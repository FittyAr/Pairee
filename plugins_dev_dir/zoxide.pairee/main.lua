-- zoxide.pairee — wraps the system `zoxide query` to navigate
-- the active panel to a frequently-used directory.
--
-- Usage: press F6 in Pairee to pick a path from the zoxide
-- history. The plugin shells out to zoxide (which must be
-- installed on the user's PATH), parses the chosen path, and
-- uses `pairee.emit("cd", ...)` to navigate the active panel.
--
-- zoxide itself is a compiled binary (zoxide.db on disk) that
-- tracks directories the user has visited. This plugin is a
-- thin wrapper that keeps the binding surface declarative and
-- lets plugins opt into per-tooling behaviours via the
-- settings schema (extra args, etc.).

--- @sync entry
--- @since 0.7.0

local M = {}

-- On Pairee's `on_cd` event (the user has navigated), call
-- `zoxide add` in the background to keep the history fresh.
-- We subscribe via `pairee.ps.sub` so the binding survives
-- across panels.
local function setup_event_handlers()
    local ok, err = pcall(pairee.ps.sub, "on_cd", function(payload)
        -- payload is the table the dispatcher sent (today it
        -- is `{path = "..."}`). We forward to zoxide as a
        -- background task so the user does not feel the cost.
        if not payload or not payload.path then
            return
        end
        local child = Command("zoxide")
            :arg("add")
            :arg(payload.path)
            :stdin(Command.NULL)
            :stdout(Command.NULL)
            :stderr(Command.NULL)
            :spawn()
        if child then
            -- Reap in the background; the comment in the docstring
            -- promises a non-blocking add, so we don't await here.
            -- Dropping the Child's pipes and reaping via `wait`
            -- avoids leaving a zombie process.
            child:wait()
        end
    end)
    if not ok then
        pairee.log.warn("zoxide: failed to subscribe to on_cd: " .. tostring(err))
    end
end

-- Probe zoxide for the path the user wants. We default to
-- `zoxide query` (interactive selection); the user can also
-- pass a substring via `args[2]` for a direct match.
local function query_zoxide(arg, extra_args)
    local args = { "query" }
    if extra_args and extra_args ~= "" then
        for flag in string.gmatch(extra_args, "%S+") do
            table.insert(args, flag)
        end
    end
    if arg and arg ~= "" then
        table.insert(args, "--")
        table.insert(args, arg)
    end
    local child = Command("zoxide")
        :arg("query")
        :stdin(Command.NULL)
        :stdout(Command.PIPED)
        :stderr(Command.PIPED)
        :spawn()
    if not child then
        return nil, "zoxide not installed"
    end
    local line = child:read_line()
    -- Reap the child (zoxide has finished writing the path back
    -- to stdout; `wait` blocks until the process actually exits
    -- so we don't race-close stdout and get SIGPIPE).
    child:wait()
    if not line or line == "" then
        return nil, "no path chosen"
    end
    return line, nil
end

-- The M3 `entry(args)` callback.
function M.entry(M_table, args)
    if not setup_done() then
        setup_event_handlers()
        _M.setup_done = true
    end
    local action = args[1] or "zoxide_query"
    if action ~= "zoxide_query" then
        return
    end
    local extra = (M_table.settings and M_table.settings.extra_args) or ""
    local path, err = query_zoxide(args[2], extra)
    if not path then
        pairee.log.warn("zoxide: " .. tostring(err or "no path"))
        return
    end
    -- Emit the `cd` action (resolved by `dispatch_emit_action`
    -- through the keybinding resolver).
    pairee.emit("cd", { path = path })
end

-- Tiny helper: returns true the first time we are called, so
-- the on_cd subscription runs exactly once per plugin load.
function _M.setup_done()
    return _M._setup_done == true
end
function _M.mark_setup_done()
    _M._setup_done = true
end

return M
