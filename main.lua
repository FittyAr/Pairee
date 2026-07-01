-- Pairee Plugin Entry
local plugin = {}

function plugin.setup(opts)
    -- Use pairee.t to fetch localized strings from lang/<locale>.toml
    pairee.log.info(pairee.t("setup.welcome"))
end

-- Custom Command Entry
function plugin.entry(args)
    local msg = pairee.t("command.executed", { count = tostring(#args) })
    pairee.app.notify(pairee.t("command.title"), msg, "info")
end

-- Custom Previewer
function plugin.peek(job)
    local preview_msg = pairee.t("preview.file_path", { path = job.file.path })
    return pairee.ui.Paragraph(preview_msg)
end

return plugin
