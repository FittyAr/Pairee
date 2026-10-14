--- @since 1.3.0
--- @sync entry
-- Acceptance port of an fzf plugin: pipes the selection (or the files of
-- the current folder) into a picker through `pairee.Command` with piped
-- stdio and jumps to the picked entry. The picker is `settings.bin`
-- (`fzf` by default; the test uses `sort`, which also reads lines on
-- stdin and writes lines on stdout).
local M = {}

--- Paths offered to the picker: the selection, else the current folder.
function M.candidates(cx)
    local paths = {}
    if #cx.active.selected > 0 then
        for _, file in ipairs(cx.active.selected) do
            paths[#paths + 1] = file.path
        end
    else
        for _, file in ipairs(pairee.fs.read_dir(cx.active.cwd)) do
            paths[#paths + 1] = file.path
        end
    end
    return paths
end

--- Arguments for the picker (only fzf gets fzf's options).
function M.args(bin)
    if bin == "fzf" then
        return { "--multi", "--height", "100%" }
    end
    return {}
end

--- Runs `bin` with `lines` on stdin; returns the lines it printed.
function M.pick(bin, lines)
    local Command = pairee.Command
    local child = Command(bin)
        :arg(M.args(bin))
        :stdin(Command.PIPED)
        :stdout(Command.PIPED)
        :stderr(Command.NULL)
        :spawn()
    -- fzf reads LF everywhere; other Windows tools (sort.exe) want CRLF.
    local newline = "\n"
    if bin ~= "fzf" and pairee.utils.target_family() == "windows" then
        newline = "\r\n"
    end
    child:write_all(table.concat(lines, newline) .. newline)
    child:close_stdin()
    local output = child:wait_with_output()
    local picked = {}
    for line in output.stdout:gmatch("[^\r\n]+") do
        picked[#picked + 1] = line
    end
    return picked, output.status.success
end

--- The folder to open for a picked path (a file opens its parent).
function M.target(path, is_dir)
    if is_dir then
        return path
    end
    return tostring(pairee.Path(path).parent)
end

function M:entry()
    local bin = pairee.settings.bin or "fzf"
    local picked = M.pick(bin, M.candidates(pairee.cx))
    if #picked == 1 then
        local file = pairee.fs.file(picked[1])
        pairee.emit("cd", { path = M.target(picked[1], file.is_dir) })
    end
end

return M
