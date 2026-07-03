-- Pairee FZF Plugin
local plugin = {}

function plugin.setup(opts)
    pairee.log.info("FZF finder plugin loaded")
end

function plugin.entry(args)
    -- launch fzf in terminal overlay
    pairee.app.notify("FZF Finder", "Searching for files in current path...", "info")
end

return plugin
