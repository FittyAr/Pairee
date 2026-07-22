-- code-preview.pairee — syntax-highlighted code preview.
--
-- M4-T11 acceptance plugin: demonstrates the `pairee.preview_code`
-- API (built on `syntect`) for inline syntax highlighting in the
-- preview pane.
--
-- Usage:
--   - Move the cursor to a code file (.rs, .py, .js, .ts, .go,
--     .java, .c, .cpp, .toml, .json, .md, ...).
--   - Press F8 to render a syntax-highlighted preview.
--   - The preview is sent through `pairee.preview_widget(opts, text)`
--     which dispatches to the preview pane.

--- @since 0.7.0

local M = {}

-- The list of file extensions that we treat as "code". Other
-- extensions get a fallback (plain text) preview.
local CODE_EXTS = {
    rs = true, py = true, js = true, mjs = true, ts = true,
    go = true, java = true, c = true, cpp = true, cc = true, h = true,
    hpp = true, toml = true, json = true, xml = true, html = true,
    css = true, sh = true, bash = true, yml = true, yaml = true,
    md = true, sql = true, lua = true, rs = true, swift = true,
    kt = true, kts = true, dart = true, ex = true, exs = true,
    rb = true, pyw = true, php = true, pl = true, vim = true,
}

local function is_code(path)
    local ext = path:match("%.([^%.]+)$")
    if not ext then return false end
    return CODE_EXTS[ext:lower()] == true
end

-- The plugin's `peek(job)` handler is the canonical entry point
-- for previewers. It receives a job with `file.url`, `file.path`,
-- `area.width/height`, and `skip`. We dispatch to preview_code
-- when the file extension matches one we know about.
function M.peek(M_table, job)
    local path = job.file and job.file.path or job.file_url or ""
    if not is_code(path) then
        return nil
    end

    local ok, text = pcall(pairee.preview_code, { path = path })
    if not ok then
        return nil
    end

    -- `text` is a `ui.Text` userdata. Hand it to the preview pane
    -- via `preview_widget` so the rich-rendering path renders it.
    -- The opts table can carry additional knobs (skip, area) in
    -- future versions.
    return {
        type = "Renderable",
        renderable = text,
    }
end

-- An optional `entry(args)` handler that uses `preview_widget`
-- directly (rather than the peek path) for plugin authors who
-- want a one-shot preview.
function M.entry(M_table, args)
    local action = args[1]
    if action ~= "preview_code" then
        return
    end
    local path = args[2]
    if not path then
        return
    end
    if not is_code(path) then
        return
    end
    local ok, text = pcall(pairee.preview_code, { path = path })
    if not ok then return end
    -- The preview_widget binding accepts a Text userdata; it
    -- converts to PluginWidget::RichText and dispatches to the
    -- preview pane. The preview pane renders it via the rich
    -- widget path in quickview.rs.
    pairee.preview_widget({ path = path }, text)
end

return M
