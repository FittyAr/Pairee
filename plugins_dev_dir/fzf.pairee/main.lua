-- fzf.pairee — fuzzy file navigation powered by the system fzf.
--
-- Usage: from Pairee, press F5 to launch the picker against the
-- active panel's current directory. The plugin reads `pairee.cx`
-- (M3 slim snapshot) for the cwd, pipes every entry under that
-- directory into fzf, parses the user's selection, and uses
-- `pairee.emit` to navigate the panel (`cd` for a directory,
-- `reveal` to open a file in the other panel).
--
-- The M3 surface is intentionally small: `Command`/`Child` for
-- fzf streaming, `pairee.cx.active.current.cwd` for the start
-- directory, and `pairee.emit` for the action. The full
-- preview-pane hookup (`cx.active.selected`) lands in M4.

--- @sync entry
--- @since 0.7.0

local M = {}

-- Read the active panel's current directory from the M3 slim cx
-- snapshot. The plugin falls back to the user's home directory
-- if the snapshot is missing (e.g. when run from a sync
-- context that hasn't built cx yet).
local function active_cwd()
    local ok, cwd = pcall(function()
        return pairee.cx.active.current.cwd
    end)
    if ok and type(cwd) == "string" and cwd ~= "" then
        return cwd
    end
    return os.getenv("HOME") or "/"
end

-- Enumerate every regular file under the given directory using
-- `pairee.fs.read_dir` (M3) and pipe the paths to fzf. We rely
-- on fzf's own filtering so we do not pre-glob in Lua.
local function run_fzf(cwd, extra_args)
    local args = { "--filter=", "--no-sort", "--read0" }
    if extra_args and extra_args ~= "" then
        -- Split on whitespace to allow the user to pass any fzf
        -- flag they want (e.g. `--height=40%`).
        for flag in string.gmatch(extra_args, "%S+") do
            table.insert(args, flag)
        end
    end

    local child = Command("fzf")
        :arg("--read0")
        :arg("--print0")
        :arg("--no-sort")
        :stdin(Command.PIPED)
        :stdout(Command.PIPED)
        :stderr(Command.NULL)
        :cwd(cwd)
        :spawn()

    local entries = {}
    local ok, list = pcall(pairee.fs.read_dir, cwd, { limit = 2000 })
    if ok and type(list) == "table" then
        entries = list
    end

    -- Push the entries to fzf's stdin (null-terminated).
    for _, entry in ipairs(entries) do
        local path = entry:path()
        if path then
            child:write_all(path .. "\0")
        end
    end
    -- Close stdin to signal EOF. We `take_stdin` and close the
    -- returned `ChildInput` wrapper, which drops only the stdin
    -- pipe (NOT stdout) — otherwise fzf would receive SIGPIPE
    -- when it tries to write the picked path back.
    local stdin_wrap = child:take_stdin()
    if stdin_wrap then
        stdin_wrap:close()
    end

    -- Read fzf's stdout (the picked path).
    local picked = child:read_line()
    if not picked or picked == "" then
        return nil
    end
    -- Strip the trailing null byte.
    return picked:gsub("\0$", "")
end

-- The M3 `entry(args)` callback. The first argument is the
-- configured action name (e.g. `"fuzzy_find"`).
function M.entry(M_table, args)
    local action = args[1] or "fuzzy_find"
    if action ~= "fuzzy_find" then
        return
    end

    local cwd = active_cwd()
    local extra = (M_table.settings and M_table.settings.extra_args) or ""
    local picked = run_fzf(cwd, extra)
    if not picked then
        return
    end

    -- Determine whether the pick is a directory or a file by
    -- asking `pairee.fs.cha`. In M3 (with the new fs.* ops)
    -- this returns a Cha userdata whose `is_dir` method is the
    -- canonical answer.
    local is_dir = false
    local ok, cha = pcall(pairee.fs.cha, picked, false)
    if ok and cha and cha:is_dir() then
        is_dir = true
    end

    if is_dir then
        -- `cd` is recognised by the keybinding resolver and
        -- routes to the existing Pairee Cd action (per M0
        -- `dispatch_emit_action`).
        pairee.emit("cd", { path = picked })
    else
        -- `reveal` opens the file in the inactive panel.
        pairee.emit("reveal", { url = picked })
    end
end

return M
