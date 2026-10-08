-- Pairee — disk-usage.pairee
-- Command plugin that analyses the disk usage of the active panel's working
-- directory and renders a sorted "what is eating my space?" report.
--
-- Demonstrates:
--   * The `entry()` command contract.
--   * Cross-platform scanning: `du` on POSIX, a native `pairee.fs.read_dir`
--     walk on Windows (no shell, so it also works in Secure Mode).
--   * Async ergonomics (we don't block the UI while the scan runs).
--   * Building rich widgets (`pairee.ui.Table`, `pairee.ui.Paragraph`).
--   * Localised notifications via `pairee.t()`.
--   * Reading user settings via `pairee.settings.*`.
--
-- Requires `trusted = true` (the plugin spawns `du` on POSIX). In Secure
-- Mode `du` must be declared in `manifest.toml` under
-- `[permissions] commands`.

local M = {}

---------------------------------------------------------------------------
-- Platform detection
---------------------------------------------------------------------------

local function is_windows()
    if pairee.utils and pairee.utils.target_os then
        local ok, os_name = pcall(pairee.utils.target_os)
        if ok and type(os_name) == "string" and os_name ~= "" then
            return os_name:lower():match("windows") ~= nil
        end
    end
    -- `rt.os` (when available) or a fallback string match.
    if rt and rt.os and rt.os ~= "" then
        return rt.os:lower():match("windows") ~= nil
    end
    return package.config:sub(1, 1) == "\\"
end

-- POSIX: `du` invocation with depth control.
-- Returns (cmd, args). `cwd` is appended as the last argument.
local function du_command(cwd, settings)
    local depth = tonumber(settings.depth) or 2
    -- `-d` is accepted by both GNU and BSD/macOS du (`--max-depth` is GNU-only).
    local args = { "-k", "-d", tostring(depth) }
    if settings.include_hidden then
        args[#args + 1] = "--apparent-size"
    end
    local extra = settings.extra_args or ""
    for token in extra:gmatch("%S+") do
        args[#args + 1] = token
    end
    args[#args + 1] = cwd
    return "du", args
end

-- Windows: measure every top-level entry of `cwd` with a native recursive
-- walk over `pairee.fs.read_dir` (symlinks are not followed). Returns a list
-- of { path, bytes }.
local function list_dir(path)
    local ok, items = pcall(pairee.fs.read_dir, path)
    if ok and type(items) == "table" then
        return items
    end
    return {}
end

local function tree_bytes(file, include_hidden)
    if file.is_symlink then
        return 0
    end
    if not file.is_dir then
        return tonumber(file.size) or 0
    end
    local total = 0
    for _, child in ipairs(list_dir(file.path)) do
        if include_hidden or not child.is_hidden then
            total = total + tree_bytes(child, include_hidden)
        end
    end
    return total
end

local function native_scan(cwd, settings)
    local include_hidden = settings.include_hidden and true or false
    local out = {}
    for _, item in ipairs(list_dir(cwd)) do
        if include_hidden or not item.is_hidden then
            local bytes = tree_bytes(item, include_hidden)
            if bytes > 0 then
                out[#out + 1] = { path = tostring(item.path), bytes = bytes }
            end
        end
    end
    return out
end

-- Parse the "<bytes>\t<path>" or "<kbytes>\t<path>" output we asked the
-- tool to emit. Returns a list of { path, bytes }.
local function parse_du(stdout)
    local out = {}
    for line in stdout:gmatch("[^\n]+") do
        local size, path = line:match("^%s*(%d+)%s+(.+)$")
        if not size then
            size, path = line:match("^(%d+)\t(.+)$")
        end
        if size and path then
            out[#out + 1] = {
                path  = path,
                bytes = tonumber(size) or 0,
            }
        end
    end
    return out
end

local function human_bytes(n)
    if not n or n <= 0 then return "0 B" end
    local units = { "B", "KB", "MB", "GB", "TB" }
    local i = 1
    while n >= 1024 and i < #units do
        n = n / 1024
        i = i + 1
    end
    return string.format(i == 1 and "%d %s" or "%.1f %s", n, units[i])
end

-- Trim the report to the top N entries by size.
local function top_n_entries(entries, n)
    table.sort(entries, function(a, b) return a.bytes > b.bytes end)
    if #entries > n then
        return { table.unpack(entries, 1, n) }
    end
    return entries
end

-- Render the report as a Pairee table widget.
local function render_report(entries, total, settings)
    local top = top_n_entries(entries, tonumber(settings.top_n) or 20)

    local header = { "Size", "Path" }
    local rows = { header }
    local grand_total = 0
    for _, e in ipairs(entries) do grand_total = grand_total + e.bytes end

    for _, e in ipairs(top) do
        local pct = grand_total > 0 and (e.bytes / grand_total * 100) or 0
        rows[#rows + 1] = {
            string.format("%s (%.0f%%)", human_bytes(e.bytes), pct),
            e.path,
        }
    end

    local summary = string.format(
        "%s\nScanned: %d entries · Total: %s · Depth: %d\n\nLargest %d entries:",
        total.cwd, #entries, human_bytes(grand_total),
        tonumber(settings.depth) or 2, #top
    )
    return pairee.ui.Paragraph(summary), pairee.ui.Table(header, { unpack(rows, 2) })
end

---------------------------------------------------------------------------
-- Command entry point (Ctrl+D)
---------------------------------------------------------------------------

function M:setup(_)
    -- Nothing to set up; settings are read on demand.
end

function M:entry()
    local cwd = pairee.app.cwd()
    if not cwd or cwd == "" then
        pairee.app.notify(
            "disk-usage",
            pairee.t("messages.no_cwd"),
            "warn"
        )
        return
    end

    local settings = pairee.settings or {}
    local entries
    if is_windows() then
        entries = native_scan(tostring(cwd), settings)
    else
        local cmd, args = du_command(tostring(cwd), settings)

        -- Pre-flight: check the binary is on PATH (we use `which` from
        -- the runtime to give a helpful error if it isn't).
        if pairee.which and not pairee.which({ cands = { cmd }, silent = true }) then
            pairee.app.notify(
                "disk-usage",
                string.format(pairee.t("messages.tool_missing"), cmd),
                "error"
            )
            return
        end

        -- Run the scan in the background. We do not block the UI thread.
        local ok, scan = pcall(pairee.fs.spawn, cmd, args)
        if not ok or not scan or scan.status ~= 0 then
            local stderr = (not ok and tostring(scan))
                or (scan and scan.stderr) or "(no output)"
            pairee.log.error("disk-usage: scan failed: " .. tostring(stderr))
            pairee.app.notify("disk-usage", pairee.t("messages.scan_failed"), "error")
            return
        end

        -- `du -k` reports kilobytes.
        entries = parse_du(scan.stdout or "")
        for _, e in ipairs(entries) do e.bytes = e.bytes * 1024 end
    end

    if #entries == 0 then
        pairee.app.notify("disk-usage", pairee.t("messages.nothing_found"), "info")
        return
    end

    local summary, table_widget = render_report(
        entries, { cwd = tostring(cwd) }, settings
    )

    -- The TUI command surface: stack the summary paragraph on top of the
    -- table. Plugins don't have a single "show report" primitive, so we
    -- push the table into the preview pane and let the user scroll.
    if pairee.preview_widget then
        pairee.preview_widget({ path = tostring(cwd) }, table_widget)
    end
    pairee.app.notify(
        "disk-usage",
        string.format(pairee.t("messages.report_ready"),
            #entries, tonumber(settings.top_n) or 20),
        "info"
    )
end

return M
