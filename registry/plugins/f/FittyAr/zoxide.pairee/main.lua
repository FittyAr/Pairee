-- Pairee Zoxide Plugin
local plugin = {}

function plugin.setup(opts)
    pairee.log.info("Zoxide jumping plugin loaded")
end

function plugin.entry(args)
    -- launch interactive zoxide prompt
    pairee.app.notify("Zoxide Jump", "Querying zoxide database...", "info")
end

return plugin
