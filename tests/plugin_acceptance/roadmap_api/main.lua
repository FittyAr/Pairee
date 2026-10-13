--- @since 1.2.0
--- @sync entry
-- Acceptance: the APIs of the plugin roadmap (M1-M4) from an untrusted
-- plugin. M.run(dir) gets a folder outside the plugin's jail.
local M = {}

local function check(ok, what, failures)
    if not ok then
        failures[#failures + 1] = what
    end
end

function M.run(outside)
    local f = {}
    local ui = pairee.ui

    -- Path / Url / Cha / Error
    local p = pairee.Path("dir/sub/notes.tar.gz")
    check(p.name == "notes.tar.gz" and p.ext == "gz" and p.parent.name == "sub", "Path parts", f)
    check(p:starts_with("dir") and p:join("x").name == "x", "Path ops", f)
    local u = pairee.Url("sftp://me@box//srv/a.txt")
    check(u.scheme == "sftp" and u.domain == "me@box" and u.name == "a.txt", "Url parts", f)
    check(not u.is_regular and u.parent:join("b").name == "b", "Url ops", f)
    local c = pairee.Cha({ is_dir = true, mode = 493 })
    check(c.is_dir and (c:perm() == "drwxr-xr-x"), "Cha", f)
    local e = pairee.Error.custom("boom")
    check(tostring(e) == "boom" and ("x" .. e) == "xboom", "Error", f)

    -- pairee.state
    check(pairee.state.set("runs", (pairee.state.get("runs") or 0) + 1), "state.set", f)
    check(pairee.state.get("runs") >= 1, "state.get", f)

    -- pairee.fs.cha: outside the jail is a PermissionDenied Error (the
    -- data dir exists once pairee.state wrote to it).
    local cha, err = pairee.fs.cha(outside)
    check(cha == nil and err.kind == "PermissionDenied", "fs.cha jail", f)
    local own = pairee.fs.cha(pairee.fs.data_dir())
    check(own ~= nil and own.is_dir, "fs.cha own data dir", f)

    -- Clipboard: an untrusted plugin may not read it.
    local text, cerr = pairee.clipboard()
    check(text == nil and cerr.kind == "PermissionDenied", "clipboard read refused", f)

    -- Identity utils
    check(type(pairee.utils.host_name()) == "string", "host_name", f)
    local family = pairee.utils.target_family()
    check((family == "unix") == (pairee.utils.uid() ~= nil), "uid per platform", f)

    -- Widgets
    local layout = ui.Layout():direction(ui.Layout.VERTICAL)
        :constraints({ ui.Constraint.Length(1), ui.Constraint.Fill(1) })
        :children({
            ui.Line({ ui.Span("Title"):bold(), " ", ui.Span("x", "red") }),
            ui.Table({ ui.Row({ "a", "1" }) }):header({ "Name", "Size" })
                :block(ui.Border(ui.Edge.ALL):type(ui.Border.ROUNDED):title("T")),
        })
    check(type(layout) == "userdata", "ui.Layout", f)
    check(type(ui.Text.parse("\27[31mred\27[0m")) == "userdata", "ui.Text.parse", f)

    -- rt / th / image (lazy namespaces)
    check(pairee.rt.plugin.name == "roadmap_api" and pairee.rt.plugin.trusted == false, "rt.plugin", f)
    check(pairee.rt.api == pairee._lua_api_version, "rt.api", f)
    check(type(pairee.th.name) == "string", "th.name", f)
    check(type(pairee.image.show) == "function", "image namespace", f)
    local shown, ierr = pairee.image.show(outside)
    check(shown == nil and ierr.kind == "PermissionDenied", "image.show jail", f)

    return { ok = #f == 0, missing = table.concat(f, ",") }
end

return M
