--- @since 1.3.0
--- @sync entry
-- Pairee — disk-usage
-- Command plugin that analyses the disk usage of the active panel's folder
-- and shows a sorted "what is eating my space?" report.
--
-- Demonstrates:
--   * A `[[commands]]` entry (`report`) and the `entry(args)` contract.
--   * Reading the panels through `pairee.cx` (refreshed by `@sync entry`).
--   * Cross-platform scanning: `du` on POSIX (declared under
--     `[permissions] commands` for Secure Mode), a native
--     `pairee.fs.read_dir` walk on Windows.
--   * Composing widgets (`ui.Layout`, `ui.Paragraph`, `ui.Table`) and
--     showing them with `pairee.preview_widget`.
--   * Localised text via `pairee.t()` and settings via `pairee.settings`.
--
-- Requires `trusted = true` (the plugin spawns `du` on POSIX).

local M = {}

---------------------------------------------------------------------------
-- Scanning
---------------------------------------------------------------------------

local function is_windows()
    return pairee.rt.family == "windows"
end

local function base_name(path)
    return path:match("([^/\\]+)[/\\]*$") or path
end

local function is_hidden_name(path)
    return base_name(path):sub(1, 1) == "."
end

-- POSIX: `du -k -d <depth> [extra args] <cwd>`.
local function du_command(cwd, settings)
    local depth = tonumber(settings.depth) or 2
    -- `-d` is accepted by both GNU and BSD/macOS du (`--max-depth` is GNU-only).
    local args = { "-k", "-d", tostring(depth) }
    for token in (settings.extra_args or ""):gmatch("%S+") do
        args[#args + 1] = token
    end
    args[#args + 1] = cwd
    return "du", args
end

-- Parse the "<kbytes>\t<path>" lines of `du -k`, dropping the line of the
-- scanned folder itself (it would always be the largest entry).
local function parse_du(stdout, cwd, include_hidden)
    local out = {}
    local root = cwd:gsub("/+$", "")
    for line in stdout:gmatch("[^\n]+") do
        local size, path = line:match("^(%d+)%s+(.+)$")
        if size and path:gsub("/+$", "") ~= root
            and (include_hidden or not path:find("/%.[^/]")) then
            out[#out + 1] = { path = path, bytes = (tonumber(size) or 0) * 1024 }
        end
    end
    return out
end

-- Windows: measure every top-level entry of `cwd` with a recursive walk
-- over `pairee.fs.read_dir` (symlinks are not followed).
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

-- Returns the entries, or nil after telling the user why the folder could
-- not be read.
local function native_scan(cwd, include_hidden)
    local ok, items = pcall(pairee.fs.read_dir, cwd)
    if not ok then
        pairee.log.error("disk-usage: cannot read " .. cwd .. ": " .. tostring(items))
        pairee.app.notify("disk-usage", pairee.t("messages.scan_failed"), "error")
        return nil
    end
    local out = {}
    for _, item in ipairs(items) do
        if include_hidden or not item.is_hidden then
            local bytes = tree_bytes(item, include_hidden)
            if bytes > 0 then
                out[#out + 1] = { path = tostring(item.path), bytes = bytes }
            end
        end
    end
    return out
end

-- POSIX scan through `du`. Returns the entries, or nil after telling the
-- user why it failed.
local function du_scan(cwd, settings)
    local cmd, args = du_command(cwd, settings)
    local ok, scan = pcall(pairee.fs.spawn, cmd, args)
    if not ok then
        pairee.log.error("disk-usage: cannot run " .. cmd .. ": " .. tostring(scan))
        pairee.app.notify("disk-usage", pairee.t("messages.tool_missing", { tool = cmd }), "error")
        return nil
    end
    if scan.status ~= 0 and (scan.stdout or "") == "" then
        pairee.log.error("disk-usage: scan failed: " .. tostring(scan.stderr))
        pairee.app.notify("disk-usage", pairee.t("messages.scan_failed"), "error")
        return nil
    end
    -- du exits non-zero for unreadable folders but still lists the rest.
    return parse_du(scan.stdout or "", cwd, settings.include_hidden)
end

---------------------------------------------------------------------------
-- Report
---------------------------------------------------------------------------

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

-- The report widget: a summary paragraph above the table of the largest
-- entries.
local function render_report(entries, cwd, settings)
    local ui = pairee.ui
    local top_n = tonumber(settings.top_n) or 20

    local grand_total = 0
    for _, e in ipairs(entries) do
        grand_total = grand_total + e.bytes
    end
    table.sort(entries, function(a, b) return a.bytes > b.bytes end)

    local rows = {}
    for i = 1, math.min(top_n, #entries) do
        local e = entries[i]
        local pct = grand_total > 0 and (e.bytes / grand_total * 100) or 0
        rows[#rows + 1] = { string.format("%s (%.0f%%)", human_bytes(e.bytes), pct), e.path }
    end

    local summary = ui.Paragraph(pairee.t("messages.summary", {
        cwd = cwd,
        count = tostring(#entries),
        total = human_bytes(grand_total),
        top = tostring(#rows),
    })):wrap(ui.Wrap.YES)
    local report = ui.Table(rows)
        :header({ pairee.t("messages.header_size"), pairee.t("messages.header_path") })
        :widths({ ui.Constraint.Length(16), ui.Constraint.Fill(1) })
    return ui.Layout()
        :direction(ui.Layout.VERTICAL)
        :constraints({ ui.Constraint.Length(4), ui.Constraint.Fill(1) })
        :children({ summary, report }),
        #rows
end

---------------------------------------------------------------------------
-- Command `report`
---------------------------------------------------------------------------

function M:entry(args)
    local cwd = pairee.cx.active.cwd
    -- Local folders only: `du` and `pairee.fs` cannot measure remote panels.
    if not cwd or cwd == "" or not pairee.cx.active.url.is_regular then
        pairee.app.notify("disk-usage", pairee.t("messages.no_cwd"), "warn")
        return
    end

    local settings = pairee.settings
    local entries
    if is_windows() then
        entries = native_scan(cwd, settings.include_hidden)
    else
        entries = du_scan(cwd, settings)
    end
    if not entries then
        return
    end

    if #entries == 0 then
        pairee.app.notify("disk-usage", pairee.t("messages.nothing_found"), "info")
        return
    end

    -- Shown in the Quick View of the scanned folder.
    local widget, shown = render_report(entries, cwd, settings)
    local ok, err = pairee.preview_widget({ file = cwd }, widget)
    if not ok then
        pairee.log.warn("disk-usage: cannot show the report: " .. tostring(err))
    end
    pairee.app.notify(
        "disk-usage",
        pairee.t("messages.report_ready", { total = tostring(#entries), top = tostring(shown) }),
        "info"
    )
end

return M
