-- Pairee — disk-usage.pairee
-- Command plugin that analyses the disk usage of the active panel's working
-- directory and renders a sorted "what is eating my space?" report.
--
-- Demonstrates:
--   * The `entry()` command contract.
--   * Cross-platform spawning: `du` on POSIX, PowerShell on Windows.
--   * Async ergonomics (we don't block the UI while the scan runs).
--   * Building rich widgets (`pairee.ui.Table`, `pairee.ui.Paragraph`).
--   * Localised notifications via `pairee.t()`.
--   * Reading user settings via `pairee.settings.*`.
--
-- Requires `trusted = true` (the plugin spawns an external process).

local M = {}

---------------------------------------------------------------------------
-- Platform detection
---------------------------------------------------------------------------

local function is_windows()
    -- `rt.os` (when available) or a fallback string match.
    if rt and rt.os and rt.os ~= "" then
        return rt.os:lower():match("windows") ~= nil
    end
    return package.config:sub(1, 1) == "\\"
end

-- Pick the right `du` invocation for the current platform.
-- Returns (cmd, args). `cwd` is appended as the last argument.
local function du_command(cwd, settings)
    local extra = settings.extra_args or ""
    local extra_tokens = {}
    for token in extra:gmatch("%S+") do
        extra_tokens[#extra_tokens + 1] = token
    end

    if is_windows() then
        -- Use PowerShell's `Get-ChildItem` + measured sizes. We keep the
        -- output format compatible with the POSIX branch by emitting
        -- "<bytes>\t<path>" lines.
        local depth = tonumber(settings.depth) or 2
        local include_hidden = settings.include_hidden and "-Force" or ""
        local script = string.format([[
            Get-ChildItem -LiteralPath '%s' %s -Directory |
                ForEach-Object {
                    $bytes = (Get-ChildItem -LiteralPath $_.FullName -Recurse -File -ErrorAction SilentlyContinue |
                        Measure-Object -Property Length -Sum).Sum
                    if ($bytes) {
                        [Console]::OutputEncoding = [System.Text.Encoding]::UTF8
                        Write-Output ($bytes.ToString() + [char]9 + $_.FullName)
                    }
                }
        ]], cwd:gsub("'", "''"), include_hidden)
        return "powershell", { "-NoProfile", "-NonInteractive", "-Command", script }
    end

    -- POSIX: standard `du` with depth control.
    local depth = tonumber(settings.depth) or 2
    local args = { "-k", "--max-depth=" .. tostring(depth) }
    if settings.include_hidden then
        args[#args + 1] = "--apparent-size"
    end
    for _, t in ipairs(extra_tokens) do args[#args + 1] = t end
    args[#args + 1] = cwd
    return "du", args
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
    local scan = pairee.fs.spawn(cmd, args)
    if not scan or scan.status ~= 0 then
        local stderr = scan and scan.stderr or "(no output)"
        pairee.log.error("disk-usage: scan failed: " .. tostring(stderr))
        pairee.app.notify("disk-usage", pairee.t("messages.scan_failed"), "error")
        return
    end

    local entries = parse_du(scan.stdout or "")
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
