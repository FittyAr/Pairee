-- Acceptance: filesystem extras + File userdata against ACCEPT_ROOT.
local M = {}

local function join(root, a, b)
    if b then
        return root .. "/" .. a .. "/" .. b
    end
    return root .. "/" .. a
end

function M.run(root)
    assert(type(root) == "string" and #root > 0, "root required")

    local dir = join(root, "nested")
    pairee.fs.mkdir("dir_all", dir)

    local src = join(dir, "a.txt")
    pairee.fs.write(src, "hello")
    assert(pairee.fs.exists(src), "write did not create file")
    assert(pairee.fs.read(src) == "hello", "read mismatch")

    local copy = join(dir, "b.txt")
    local n = pairee.fs.copy(src, copy)
    assert(n == 5, "copy byte count")
    assert(pairee.fs.read(copy) == "hello", "copy content")

    local renamed = join(dir, "c.txt")
    pairee.fs.rename(copy, renamed)
    assert(pairee.fs.exists(renamed), "rename dest missing")
    assert(not pairee.fs.exists(copy), "rename source still there")

    local f = pairee.fs.file(src)
    assert(f.name == "a.txt", "File.name")
    assert(f.size == 5, "File.size")
    assert(f.is_dir == false, "File.is_dir")
    assert(tostring(f):find("a.txt", 1, true), "File tostring")

    local listed = pairee.fs.read_dir(dir)
    assert(#listed == 2, "read_dir count")

    pairee.fs.remove("file", src)
    pairee.fs.remove("dir_all", dir)
    assert(not pairee.fs.exists(dir), "dir_all leftover")

    return { ok = true, copied = n }
end

return M
