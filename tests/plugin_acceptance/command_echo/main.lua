-- Acceptance: Command builder. :output() is async; the Rust runner
-- calls M.run via call_async so the future is polled.
local M = {}

function M.run()
    local os_name = pairee.utils.target_os()
    local cmd
    if os_name == "windows" then
        cmd = pairee.Command("cmd.exe"):arg("/C"):arg("echo hello")
    else
        cmd = pairee.Command("echo"):arg("hello")
    end
    local out = cmd:stdout(pairee.Command.PIPED):stderr(pairee.Command.PIPED):output()
    local stdout = out.stdout or ""
    local ok = out.status.success and stdout:lower():find("hello", 1, true) ~= nil
    return { ok = ok, stdout = stdout }
end

function M.surface()
    local c = pairee.Command("rg"):arg("-n"):arg({ "TODO" })
    return { ok = tostring(c) == "Command(rg)" }
end

return M
