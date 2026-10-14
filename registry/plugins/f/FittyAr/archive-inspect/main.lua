--- @since 1.3.0
--- @sync entry
-- Pairee — archive-inspect
-- Previewer plugin that lists the contents of ZIP / TAR(.GZ) / 7Z archives
-- directly in the file manager preview pane, without extracting them.
--
-- Demonstrates:
--   * The `peek()` previewer contract (PgUp / PgDn scroll the table).
--   * Spawning external tools (`unzip`, `tar`, `7z`) via `pairee.fs.spawn`,
--     declared under `[permissions] commands` for Secure Mode.
--   * A `[[commands]]` entry (`summary`) reading `pairee.cx` with `@sync`.
--   * Reading user settings via `pairee.settings.*`.
--   * Localised text via `pairee.t()`.
--
-- Requires `trusted = true` (the plugin spawns `unzip`/`tar`/`7z`).

local M = {}

---------------------------------------------------------------------------
-- Helpers
---------------------------------------------------------------------------

-- Detect the archive kind from a path. Returns "zip", "tar", "7z" or nil.
local function detect_kind(path)
    local lower = path:lower()
    if lower:match("%.zip$") then
        return "zip"
    elseif lower:match("%.tar%.gz$") or lower:match("%.tgz$") or lower:match("%.tar$") then
        return "tar"
    elseif lower:match("%.7z$") then
        return "7z"
    end
    return nil
end

-- The listing tool for an archive kind and its base arguments.
local function tool_for(kind)
    if kind == "zip" then
        return "unzip", { "-l" }
    elseif kind == "tar" then
        -- `-tf` lets tar auto-detect gzip, so plain `.tar` works too.
        return "tar", { "-tf" }
    end
    return "7z", { "l", "-slt" }
end

-- Full listing invocation: base args, user extra args and the archive path.
local function listing_command(kind, path)
    local bin, args = tool_for(kind)
    local extra = pairee.settings.extra_args or ""
    for token in extra:gmatch("%S+") do
        args[#args + 1] = token
    end
    args[#args + 1] = path
    return bin, args
end

-- The lines of `text` without trailing carriage returns, blank lines kept.
local function lines(text)
    local out = {}
    for line in (text .. "\n"):gmatch("([^\n]*)\n") do
        out[#out + 1] = (line:gsub("\r$", ""))
    end
    return out
end

-- Parse the raw output of the listing tool into a list of entries.
-- Each entry: { path = string, size = integer, date = string }
local function parse_listing(kind, stdout)
    local entries = {}

    if kind == "zip" then
        -- `unzip -l`:   1234  2024-01-02 03:04   file.txt
        for _, line in ipairs(lines(stdout)) do
            local size, date, time, name =
                line:match("^%s*(%d+)%s+([%d%-/]+)%s+(%d%d:%d%d)%s+(.+)$")
            if size and name then
                entries[#entries + 1] = {
                    path = name,
                    size = tonumber(size) or 0,
                    date = date .. " " .. time,
                }
            end
        end
    elseif kind == "tar" then
        -- `tar -tf` prints one path per line, without sizes or dates.
        for _, line in ipairs(lines(stdout)) do
            if line ~= "" and not line:match("^%.?/?$") then
                entries[#entries + 1] = { path = line, size = 0, date = "" }
            end
        end
    else
        -- `7z l -slt` prints the archive's own block, a "----------" line,
        -- then one blank-line separated block of `Key = value` per entry.
        local started, current = false, nil
        local function finish()
            if current and current.path then
                entries[#entries + 1] = current
            end
            current = nil
        end
        for _, line in ipairs(lines(stdout)) do
            if not started then
                started = line:match("^%-%-%-%-%-%-%-%-%-%-") ~= nil
            elseif line == "" then
                finish()
            else
                local k, v = line:match("^%s*(%w+)%s*=%s*(.*)$")
                if k then
                    current = current or { size = 0, date = "" }
                    if k == "Path" then
                        current.path = v
                    elseif k == "Size" then
                        current.size = tonumber(v) or 0
                    elseif k == "Modified" then
                        current.date = v
                    end
                end
            end
        end
        finish()
    end

    return entries
end

-- Apply the user-configured filter / sort / truncation.
local function shape_entries(entries, settings)
    local out = {}
    for _, e in ipairs(entries) do
        local hidden = e.path:match("^%.[^/]") or e.path:match("/%.[^/]")
        if settings.show_hidden or not hidden then
            out[#out + 1] = e
        end
    end

    local sort_by = settings.sort_by or "path"
    table.sort(out, function(a, b)
        if sort_by == "size" then
            return a.size > b.size
        elseif sort_by == "date" then
            return a.date > b.date
        end
        return a.path < b.path
    end)

    local limit = tonumber(settings.max_entries) or 500
    if #out > limit then
        out = { table.unpack(out, 1, limit) }
    end
    return out
end

-- Format a byte count as a short human-readable string.
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

-- Whether a File lives on the local disk (the listing tools cannot read
-- remote URLs).
local function is_local(file)
    return pairee.Url(file.url).is_regular
end

-- Run the listing tool. Returns the entries, or nil and the tool name.
local function list_archive(kind, path)
    local bin, args = listing_command(kind, path)
    local ok, result = pcall(pairee.fs.spawn, bin, args)
    if not ok or result.status ~= 0 then
        local reason = ok and result.stderr or tostring(result)
        pairee.log.warn(string.format("archive-inspect: %s failed on %s: %s", bin, path, reason))
        return nil, bin
    end
    return parse_listing(kind, result.stdout or "")
end

---------------------------------------------------------------------------
-- Previewer contract
---------------------------------------------------------------------------

function M:peek(job)
    local file = job.file
    if file.is_dir or not is_local(file) then
        return nil
    end
    local path = tostring(file.path)
    local kind = detect_kind(path)
    if not kind then
        return nil
    end

    local ui = pairee.ui
    local entries, bin = list_archive(kind, path)
    if not entries then
        return ui.Paragraph(pairee.t("messages.listing_failed") .. "\n\n"
            .. pairee.t("messages.tool_hint", { tool = bin })):wrap(ui.Wrap.YES)
    end

    local shaped = shape_entries(entries, pairee.settings)
    if #shaped == 0 then
        return ui.Paragraph(pairee.t("messages.empty"))
    end

    local rows = {}
    for _, e in ipairs(shaped) do
        rows[#rows + 1] = { e.path, human_size(e.size), e.date }
    end
    return ui.Table(rows)
        :header({
            pairee.t("messages.header_path"),
            pairee.t("messages.header_size"),
            pairee.t("messages.header_modified"),
        })
        :widths({ ui.Constraint.Fill(1), ui.Constraint.Length(8), ui.Constraint.Length(19) })
end

---------------------------------------------------------------------------
-- Command `summary`: entry count and total size of the hovered archive
---------------------------------------------------------------------------

function M:entry(args)
    local title = pairee.t("messages.summary_title")
    local hovered = pairee.cx.active.hovered
    if not hovered then
        pairee.app.notify(title, pairee.t("messages.no_hovered"), "warn")
        return
    end
    local path = tostring(hovered.path)
    local kind = is_local(hovered) and detect_kind(path)
    if not kind then
        pairee.app.notify(title, pairee.t("messages.not_archive", { path = path }), "warn")
        return
    end

    local entries, bin = list_archive(kind, path)
    if not entries then
        pairee.app.notify(title, pairee.t("messages.tool_hint", { tool = bin }), "error")
        return
    end
    local total = 0
    for _, e in ipairs(entries) do
        total = total + e.size
    end
    pairee.app.notify(title, pairee.t("messages.summary", {
        name = hovered.name,
        count = tostring(#entries),
        size = human_size(total),
    }), "info")
end

return M
