-- Pairee Git Plugin
local plugin = {}

function plugin.setup(opts)
    pairee.log.info("Git integration plugin loaded")
end

function plugin.entry(args)
    pairee.app.notify("Git status updated", "Successfully refreshed status indicators", "info")
end

return plugin
