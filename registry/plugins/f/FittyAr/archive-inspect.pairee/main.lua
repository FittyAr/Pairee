-- Pairee — archive-inspect.pairee
-- Previewer plugin that lists the contents of ZIP / TAR.GZ / 7Z archives
-- directly in the file manager preview pane, without extracting them.
--
-- Demonstrates:
--   * The `peek()` / `seek()` previewer contract.
--   * Spawning external tools (`unzip`, `tar`, `7z`) via `pairee.fs.spawn`.
--   * Reading user settings via `pairee.settings.*`.
--   * Localised notifications via `pairee.t()`.
--   * Error handling with `pairee.log.warn` and `pairee.app.notify`.
--
-- Requires `trusted = true` (the plugin shell-spawns `unzip`/`tar`/`7z`).

local M = {}

---------------------------------------------------------------------------
-- Helpers
---------------------------------------------------------------------------

-- Detect the archive kind from a path/URL and the listing tool to use.
-- Returns one of: "zip", "tar", "7z" or nil when unsupported.
local function detect_kind(path)
    if not path then return nil end
    local lower = tostring(path):lower()
    if lower:match("%.zip$") then
        return "zip"
    elseif lower:match("%.tar%.gz$") or lower:match("%.tgz$") or lower:match("%.tar$") then
        return "tar"
    elseif lower:match("%.7z$") then
        return "7z"
    end
    return nil
end

-- Resolve the binary that should be invoked for a given archive kind.
-- Returns (binary, base_args) or nil when the binary is not on PATH.
local function tool_for(kind)
    if kind == "zip" then
        return "unzip", { "-l" }
    elseif kind == "tar" then
        return "tar", { "-tzf" }
    elseif kind == "7z" then
        return "7z", { "l", "-slt" }
    end
    return nil
end

-- Parse the raw output of the listing tool into a list of entries.
-- Each entry: { path = string, size = integer, date = string }
local function parse_listing(kind, stdout)
    local entries = {}

    if kind == "zip" then
        -- `unzip -l` output example:
        --   Length      Date    Time    Name
        --   ---------  ---------- -----   ----
        --       1234  2024-01-02 03:04   file.txt
        for line in stdout:gmatch("[^\n]+") do
            local size, date, time, name =
                line:match("^%s*(%d+)%s+(%d%d%d%d%-%d%d%-%d%d)%s+(%d%d:%d%d)%s+(.+)$")
            if size and name then
                entries[#entries + 1] = {
                    path = name,
                    size = tonumber(size) or 0,
                    date = date .. " " .. time,
                }
            end
        end
    elseif kind == "tar" then
        -- `tar -tzf` output is one path per line. We don't have sizes from
        -- this listing mode, so we leave them at 0 and rely on the date
        -- column being empty.
        for line in stdout:gmatch("[^\n]+") do
            if line ~= "" and not line:match("^%.?/?$") then
                entries[#entries + 1] = {
                    path = line,
                    size = 0,
                    date = "",
                }
            end
        end
    elseif kind == "7z" then
        -- `7z l -slt` emits blocks separated by blank lines; each block
        -- contains key = value pairs:
        --   Path = foo/bar.txt
        --   Size = 1234
        --   Modified = 2024-01-02 03:04:00
        local current = nil
        for line in stdout:gmatch("[^\n]+") do
            if line == "" then
                if current and current.path then
                    entries[#entries + 1] = current
                end
                current = nil
            else
                local k, v = line:match("^%s*(%w+)%s*=%s*(.+)$")
                if k and v then
                    if not current then current = {} end
                    if k == "Path" then current.path = v
                    elseif k == "Size" then current.size = tonumber(v) or 0
                    elseif k == "Modified" then current.date = v
                    end
                end
            end
        end
        if current and current.path then
            entries[#entries + 1] = current
        end
    end

    return entries
end

-- Apply user-configured filter / sort / truncation.
local function shape_entries(entries, settings)
    local out = {}
    for _, e in ipairs(entries) do
        if settings.show_hidden or not e.path:match("/%.[^/]+$") and not e.path:match("^%.[^/]+$") then
            out[#out + 1] = e
        end
    end

    local sort_by = settings.sort_by or "path"
    table.sort(out, function(a, b)
        if sort_by == "size" then
            return a.size > b.size
        elseif sort_by == "date" then
            return tostring(a.date) > tostring(b.date)
        end
        return tostring(a.path) < tostring(b.path)
    end)

    local limit = tonumber(settings.max_entries) or 500
    if #out > limit then
        out = { table.unpack(out, 1, limit) }
    end
    return out
end

-- Format an integer byte count as a short human-readable string.
local function human_size(n)
    if not n or n <= 0 then return "-" end
    local units = { "B", "K", "M", "G", "T" }
    local i = 1
    while n >= 1024 and i < #units do
        n = n / 1024
        i = i + 1
    end
    return string.format(i == 1 and "%d %s" or "%.1f %s", n, units[i])
end

---------------------------------------------------------------------------
-- Previewer contract
---------------------------------------------------------------------------

function M:peek(job)
    if not job or not job.file or not job.file.url then
        return nil
    end

    local path = tostring(job.file.url)
    local kind = detect_kind(path)
    if not kind then
        return nil -- not an archive we know
    end

    local bin, base = tool_for(kind)
    if not bin then
        return nil
    end

    -- Build the argument list, appending user-defined extra args.
    local args = { unpack(base) }
    local extra = pairee.settings.extra_args or ""
    for token in extra:gmatch("%S+") do
        args[#args + 1] = token
    end
    args[#args + 1] = path

    local result = pairee.fs.spawn(bin, args)
    if not result or result.status ~= 0 then
        pairee.log.warn(string.format("archive-inspect: %s failed on %s", bin, path))
        return pairee.ui.Paragraph(
            string.format(
                "Could not list archive contents.\n\nTool: %s\nMake sure it is installed and on PATH.",
                bin
            )
        )
    end

    local parsed = parse_listing(kind, result.stdout or "")
    local shaped = shape_entries(parsed, pairee.settings or {})

    -- Stash the shaped list on the job so seek() can paginate without
    -- re-spawning the binary.
    job._archive_entries = shaped
    job._archive_skip = tonumber(job.skip) or 0

    local rows = {
        { "Path", "Size", "Modified" },
    }
    for _, e in ipairs(shaped) do
        rows[#rows + 1] = { e.path, human_size(e.size), e.date or "" }
    end
    if #rows == 1 then
        return pairee.ui.Paragraph("Archive is empty.")
    end
    return pairee.ui.Table(rows[1], { unpack(rows, 2) })
end

function M:seek(job)
    if not job or not job._archive_entries then
        return nil
    end
    -- For previewer tables, Pairee handles scrolling internally based on
    -- job.skip. We just re-emit the table; the engine will clip to the
    -- visible window.
    local entries = job._archive_entries
    local rows = { { "Path", "Size", "Modified" } }
    for _, e in ipairs(entries) do
        rows[#rows + 1] = { e.path, human_size(e.size), e.date or "" }
    end
    return pairee.ui.Table(rows[1], { unpack(rows, 2) })
end

---------------------------------------------------------------------------
-- Optional command entry (F2): quick popup summary
---------------------------------------------------------------------------

function M:entry()
    local entry = pairee.app.hovered()
    if not entry or not entry.url then
        pairee.app.notify("archive-inspect", "No file hovered.", "warn")
        return
    end
    local path = tostring(entry.url)
    local kind = detect_kind(path)
    if not kind then
        pairee.app.notify(
            "archive-inspect",
            pairee.t("messages.not_archive", { path = path }),
            "warn"
        )
        return
    end

    -- Reuse the listing machinery.
    local bin, base = tool_for(kind)
    local result = pairee.fs.spawn(bin, base)
    if not result or result.status ~= 0 then
        pairee.app.notify("archive-inspect", "Listing failed.", "error")
        return
    end
    local entries = parse_listing(kind, result.stdout or "")
    local total = 0
    for _, e in ipairs(entries) do total = total + (e.size or 0) end

    pairee.app.notify(
        "archive-inspect",
        string.format("%s\n\n%d entries\nTotal size: %s",
            path, #entries, human_size(total)),
        "info"
    )
end

return M
