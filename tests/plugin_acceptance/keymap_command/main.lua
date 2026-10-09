-- Acceptance: a command receives its id, and the keymap is readable.
local M = {}

M.received = nil

-- `args[1]` and `args.command` name the command whose key was pressed.
function M:entry(args)
    M.received = args.command or args[1]
    if M.received == "hop" then
        pairee.emit("go_to_tab_2")
    end
end

function M.run()
    local missing = {}
    if type(pairee.keymap) ~= "table" then
        missing[#missing + 1] = "keymap"
    elseif type(pairee.keymap.list) ~= "function" or type(pairee.keymap.chord_for) ~= "function" then
        missing[#missing + 1] = "keymap.list/chord_for"
    else
        local list = pairee.keymap.list()
        if type(list) ~= "table" then
            missing[#missing + 1] = "keymap.list() table"
        end
    end
    return { ok = #missing == 0, missing = table.concat(missing, ",") }
end

return M
