--- @since 1.3.0
-- Pairee — recent-files
-- Mixed hook + command plugin that tracks the folders (and optionally the
-- files) the user visits and opens a picker to jump back to any of them.
--
-- Demonstrates:
--   * The `setup()` lifecycle hook with `pairee.ps.sub` (`on_cd`, `on_hover`).
--   * Persistent plugin state with `pairee.state` (kept between sessions).
--   * A `[[commands]]` entry (`pick`) with `pairee.which` for selection.
--   * Cross-plugin pub/sub (publishes a `recent-files:added` event).
--   * Localised text via `pairee.t()` and settings via `pairee.settings`.
--
-- Runs untrusted: no processes, no Lua `os` / `io` / `load`.

local M = {}

-- Keys of the picker, in order: one per entry it can show.
local PICK_KEYS = "123456789abcdefghijklmnopqrstuvwxyz"

-- Newest first: { path = string, kind = "dir" | "file" }
local entries = {}

local function max_entries()
    local cap = tonumber(pairee.settings.max_entries) or 50
    return math.max(1, math.floor(cap))
end

local function save()
    local ok, err = pairee.state.set("entries", entries)
    if not ok then
        pairee.log.error("recent-files: cannot save the history: " .. tostring(err))
    end
end

local function load_history()
    local stored = pairee.state.get("entries")
    entries = {}
    if type(stored) ~= "table" then
        return
    end
    for _, e in ipairs(stored) do
        if type(e) == "table" and type(e.path) == "string" and e.path ~= "" then
            entries[#entries + 1] = { path = e.path, kind = e.kind == "file" and "file" or "dir" }
        end
    end
end

local function push_entry(path, kind)
    if type(path) ~= "string" or path == "" then
        return
    end
    -- Move the path to the top, dropping its earlier occurrence.
    local kept = { { path = path, kind = kind } }
    for _, e in ipairs(entries) do
        if e.path ~= path and #kept < max_entries() then
            kept[#kept + 1] = e
        end
    end
    entries = kept
    save()

    -- Tell any other plugin that cares; never fail the hook over it.
    pcall(pairee.ps.pub, "recent-files:added", { path = path, kind = kind })
end

-- The folder holding `path` (the panel can only open folders).
local function parent_of(path)
    local parent = pairee.Path(path).parent
    return parent and tostring(parent) or path
end

---------------------------------------------------------------------------
-- Lifecycle
---------------------------------------------------------------------------

function M:setup(opts)
    load_history()

    if pairee.settings.record_dirs ~= false then
        pairee.ps.sub("on_cd", function(payload)
            push_entry(payload.path, "dir")
        end)
    end

    if pairee.settings.record_hover then
        pairee.ps.sub("on_hover", function(payload)
            push_entry(payload.path, payload.is_dir and "dir" or "file")
        end)
    end

    pairee.log.info(string.format("recent-files: loaded %d entries", #entries))
end

---------------------------------------------------------------------------
-- Command `pick`
---------------------------------------------------------------------------

function M:entry(args)
    if #entries == 0 then
        pairee.app.notify(pairee.t("messages.picker_title"), pairee.t("messages.empty"), "info")
        return
    end

    local shown = math.min(#entries, #PICK_KEYS)
    local cands = {}
    for i = 1, shown do
        local e = entries[i]
        local label = e.kind == "dir" and pairee.t("messages.kind_dir") or pairee.t("messages.kind_file")
        cands[i] = { on = PICK_KEYS:sub(i, i), desc = label .. "  " .. e.path }
    end

    local pick = pairee.which({ cands = cands })
    local chosen = pick and entries[pick]
    if not chosen then
        return
    end

    local target = chosen.kind == "dir" and chosen.path or parent_of(chosen.path)
    local ok, err = pcall(pairee.app.cd, target)
    if not ok then
        pairee.app.notify(
            pairee.t("messages.picker_title"),
            pairee.t("messages.cd_failed", { path = target, error = tostring(err) }),
            "error"
        )
        return
    end
    pairee.app.notify(pairee.t("messages.picker_title"), pairee.t("messages.opened", { path = chosen.path }), "info")
end

return M
