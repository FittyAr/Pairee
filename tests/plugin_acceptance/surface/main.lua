-- Acceptance: every stable top-level binding exists and has the right type.
local M = {}

local function is_fn(v)
    return type(v) == "function"
end

function M.run()
    local missing = {}
    local function need(name, pred)
        if not pred then
            missing[#missing + 1] = name
        end
    end

    need("pairee", type(pairee) == "table")
    need("_lua_api_version", type(pairee._lua_api_version) == "string")
    need("confirm", is_fn(pairee.confirm))
    need("input", is_fn(pairee.input))
    need("which", is_fn(pairee.which))
    need("notify", is_fn(pairee.notify))
    need("emit", is_fn(pairee.emit))
    need("file_cache", is_fn(pairee.file_cache))
    need("sync", is_fn(pairee.sync))
    need("t", is_fn(pairee.t))

    need("app", type(pairee.app) == "table")
    need("fs", type(pairee.fs) == "table")
    need("ui", type(pairee.ui) == "table")
    need("ps", type(pairee.ps) == "table")
    need("log", type(pairee.log) == "table")
    need("utils", type(pairee.utils) == "table")
    need("cx", type(pairee.cx) == "table")
    need("Command", type(pairee.Command) == "table")

    need("fs.read", is_fn(pairee.fs.read))
    need("fs.write", is_fn(pairee.fs.write))
    need("fs.exists", is_fn(pairee.fs.exists))
    need("fs.stat", is_fn(pairee.fs.stat))
    need("fs.list", is_fn(pairee.fs.list))
    need("fs.read_dir", is_fn(pairee.fs.read_dir))
    need("fs.file", is_fn(pairee.fs.file))
    need("fs.mkdir", is_fn(pairee.fs.mkdir))
    need("fs.remove", is_fn(pairee.fs.remove))
    need("fs.rename", is_fn(pairee.fs.rename))
    need("fs.copy", is_fn(pairee.fs.copy))
    need("fs.spawn", is_fn(pairee.fs.spawn))

    need("utils.target_os", is_fn(pairee.utils.target_os))
    need("utils.target_family", is_fn(pairee.utils.target_family))
    need("utils.time", is_fn(pairee.utils.time))
    need("utils.hash", is_fn(pairee.utils.hash))
    need("utils.quote", is_fn(pairee.utils.quote))

    need("Command.PIPED", pairee.Command.PIPED ~= nil)
    need("Command.NULL", pairee.Command.NULL ~= nil)
    need("Command.INHERIT", pairee.Command.INHERIT ~= nil)
    need("cx.active", type(pairee.cx.active) == "table")

    return {
        ok = #missing == 0,
        missing = table.concat(missing, ","),
        api = pairee._lua_api_version,
    }
end

return M
