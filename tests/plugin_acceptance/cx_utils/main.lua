-- Acceptance: live-ish cx defaults + utils (os, hash, quote, percent).
local M = {}

function M.run()
    local os_name = pairee.utils.target_os()
    local family = pairee.utils.target_family()
    local now = pairee.utils.time()
    local h1 = pairee.utils.hash("payload")
    local h2 = pairee.utils.hash("payload")
    local quoted = pairee.utils.quote("a b")
    local enc = pairee.utils.percent_encode("a b")

    assert(type(os_name) == "string" and #os_name > 0)
    assert(family == "windows" or family == "unix" or family == "wasm")
    assert(type(now) == "number" and now > 0)
    assert(h1 == h2)
    assert(type(quoted) == "string" and #quoted > 0)
    assert(enc:find("a", 1, true))

    assert(type(pairee.cx.active.cwd) == "string")
    assert(pairee.cx.active.hovered == nil)
    assert(type(pairee.cx.active.selected) == "table")
    assert(type(pairee.cx.active.current) == "table")
    assert(type(pairee.cx.left.cwd) == "string")
    assert(type(pairee.cx.right.cwd) == "string")

    return { ok = true, os = os_name, family = family }
end

return M
