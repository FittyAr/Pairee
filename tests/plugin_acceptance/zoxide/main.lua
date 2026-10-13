--- @since 1.3.0
--- @sync entry
-- Acceptance port of a zoxide-like plugin, in pure Lua: it records the
-- folders the panels enter (`on_cd` host events, checked through the
-- sender metadata so another plugin cannot fake visits), keeps them in
-- `pairee.state` and jumps to the best match of the typed words.
local M = {}

local HOUR = 3600

local function folders()
    return pairee.state.get("folders") or {}
end

--- Records a visit of `path`.
function M.visit(path, now)
    local all = folders()
    local entry = all[path] or { rank = 0, last = 0 }
    entry.rank = entry.rank + 1
    entry.last = now or pairee.utils.time()
    all[path] = entry
    assert(pairee.state.set("folders", all))
end

--- Frecency as zoxide weighs it: recent visits count more.
function M.score(entry, now)
    local age = now - entry.last
    if age < HOUR then
        return entry.rank * 4
    elseif age < 24 * HOUR then
        return entry.rank * 2
    elseif age < 7 * 24 * HOUR then
        return entry.rank / 2
    end
    return entry.rank / 4
end

--- The best folder whose path contains every word (case-insensitive),
--- other than `current`.
function M.query(words, current, now)
    now = now or pairee.utils.time()
    local best, best_score
    for path, entry in pairs(folders()) do
        local lower = path:lower()
        local matches = path ~= current
        for _, word in ipairs(words) do
            matches = matches and lower:find(word:lower(), 1, true) ~= nil
        end
        local score = M.score(entry, now)
        if matches and (best_score == nil or score > best_score) then
            best, best_score = path, score
        end
    end
    return best
end

function M.setup()
    pairee.ps.sub("on_cd", function(data, meta)
        -- Only Pairee itself reports folder changes.
        if meta and meta.from_host and data and data.path then
            M.visit(data.path)
        end
    end)
end

function M:entry(args)
    local words = {}
    for i = 2, #args do
        words[#words + 1] = args[i]
    end
    local target = M.query(words, pairee.cx.active.cwd)
    if target then
        pairee.emit("cd", { path = target })
    end
end

return M
