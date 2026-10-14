--- @since 1.3.0
--- @sync entry
-- Pairee plugin entry point.
--
-- `@since` is the Lua API version the plugin needs (Pairee refuses to load
-- it on an older API); `@sync entry` refreshes `pairee.cx` before `entry`
-- runs. Pairee calls every method with the plugin table first, so define
-- them with `:` (`function M:entry(args)`).
local M = {}

function M:setup(opts)
    -- Use pairee.t to fetch localized strings from lang/<locale>.toml
    if pairee.settings.greet_on_setup then
        pairee.log.info(pairee.t("setup.welcome"))
    end
end

-- Runs from the keys or the command palette entry of a [[commands]] item;
-- args.command is its id.
function M:entry(args)
    if args.command == "hello" then
        local hovered = pairee.cx.active.hovered
        local name = hovered and hovered.name or pairee.cx.active.cwd
        pairee.app.notify(pairee.t("command.title"), pairee.t("command.hello", { name = name }), "info")
    end
end

-- Custom previewer: return a widget, or nil to leave the file to the next
-- previewer and the built-in Quick View.
function M:peek(job)
    if not job.file.name:match("%.example$") then
        return nil
    end
    local text = pairee.t("preview.file_path", { path = tostring(job.file.url) })
    return pairee.ui.Paragraph(text):wrap(pairee.ui.Wrap.YES)
end

return M
