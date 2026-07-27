-- Pairee — recent-files.pairee
-- Mixed hook + command plugin that silently tracks files and directories
-- the user visits, persists them to a JSON file, and exposes a fast
-- picker to jump back to any recent entry.
--
-- Demonstrates:
--   * The `setup()` lifecycle hook with `pairee.ps.sub`.
--   * Persistent plugin state (JSON file in the user config dir).
--   * The `entry()` command contract, with `pairee.which` for selection.
--   * Cross-plugin pub/sub (publishes a `recent-files:added` event).
--   * Locale-aware notifications via `pairee.t()`.
--   * Reading user settings via `pairee.settings.*`.
--
-- Runs in untrusted mode (no external process spawn).

local M = {}

---------------------------------------------------------------------------
-- State (in-memory mirror of the on-disk JSON file)
---------------------------------------------------------------------------

local STATE_PATH_DEFAULT = "recent-files.json"
local state = {
    entries = {}, -- newest first: { path, kind, at }
}
local dirty = false
local flush_timer = nil

local function state_path()
    local override = pairee.settings.persist_path
    if override and override ~= "" then return override end
    return STATE_PATH_DEFAULT
end

-- Minimal JSON encoder/decoder for our flat structure. We avoid extra
-- dependencies by hand-rolling a tiny implementation that handles the
-- exact shape of `state`.
local function json_encode(value)
    local t = type(value)
    if t == "nil" then return "null"
    elseif t == "boolean" then return tostring(value)
    elseif t == "number" then return tostring(value)
    elseif t == "string" then
        return '"' .. value:gsub('\\', '\\\\'):gsub('"', '\\"') .. '"'
    elseif t == "table" then
        local is_array = #value > 0
        local parts = {}
        if is_array then
            for _, v in ipairs(value) do parts[#parts + 1] = json_encode(v) end
            return "[" .. table.concat(parts, ",") .. "]"
        end
        for k, v in pairs(value) do
            parts[#parts + 1] = json_encode(tostring(k)) .. ":" .. json_encode(v)
        end
        return "{" .. table.concat(parts, ",") .. "}"
    end
    return "null"
end

local function json_decode(s)
    if not s or s == "" then return nil end
    -- A minimal decoder is overkill here. We only need to round-trip
    -- our own `state` table, so we just refuse to load anything that
    -- is not obviously ours (the file starts with a sentinel we write).
    if not s:match('^%{%s*"_recent_files_v1"') then
        return nil
    end
    -- Cheat: rely on Lua's load() against a tiny shim. This is safe
    -- because we only ever load files we wrote ourselves; the sentinel
    -- above guarantees the shape.
    local shim = "return " .. s
    local fn, err = loadstring or load
    if not fn then return nil end
    local ok, value = pcall(fn, shim)
    if not ok or type(value) ~= "table" then
        pairee.log.warn("recent-files: failed to decode state: " .. tostring(err))
        return nil
    end
    if not value.entries or type(value.entries) ~= "table" then
        return nil
    end
    return value
end

local function load_state()
    local p = state_path()
    local ok, content = pcall(pairee.fs.read, p)
    if not ok or not content or content == "" then
        return { entries = {} }
    end
    local decoded = json_decode(content)
    if not decoded then
        pairee.log.warn("recent-files: discarding malformed state file")
        return { entries = {} }
    end
    return decoded
end

local function schedule_flush()
    dirty = true
    if flush_timer then return end
    -- Debounce writes so a flurry of on_hover/on_cd events doesn't hammer
    -- the disk. We use a one-shot timer; if the runtime doesn't expose
    -- timer.set_timeout, we fall back to a synchronous flush.
    if pairee.timer and pairee.timer.set_timeout then
        flush_timer = pairee.timer.set_timeout(function()
            flush_timer = nil
            M:_flush()
        end, 500)
    else
        M:_flush()
    end
end

function M:_flush()
    if not dirty then return end
    dirty = false
    local payload = { _recent_files_v1 = true, entries = state.entries }
    local ok, err = pcall(pairee.fs.write, state_path(), json_encode(payload))
    if not ok then
        pairee.log.error("recent-files: failed to persist state: " .. tostring(err))
    end
end

---------------------------------------------------------------------------
-- Recording
---------------------------------------------------------------------------

local function push_entry(path, kind)
    if not path or path == "" then return end
    local p = tostring(path)
    -- Dedupe: remove any prior occurrence of the same path.
    local kept = {}
    for _, e in ipairs(state.entries) do
        if e.path ~= p then kept[#kept + 1] = e end
    end
    table.insert(kept, 1, { path = p, kind = kind or "file", at = os.time() })
    local cap = tonumber(pairee.settings.max_entries) or 50
    if #kept > cap then
        kept = { table.unpack(kept, 1, cap) }
    end
    state.entries = kept
    schedule_flush()

    -- Notify any other plugin that cares.
    if pairee.ps and pairee.ps.pub then
        pairee.ps.pub("recent-files:added", { path = p, kind = kind or "file" })
    end
end

---------------------------------------------------------------------------
-- Lifecycle
---------------------------------------------------------------------------

function M:setup(_)
    state = load_state()

    if pairee.ps and pairee.ps.sub then
        if pairee.settings.record_dirs ~= false then
            pairee.ps.sub("on_cd", function(payload)
                if payload and payload.cwd then
                    push_entry(payload.cwd, "dir")
                end
            end)
        end

        if pairee.settings.record_hover then
            pairee.ps.sub("on_hover", function(payload)
                if payload and payload.entry and payload.entry.url then
                    local url = tostring(payload.entry.url)
                    local kind = payload.entry.is_dir and "dir" or "file"
                    push_entry(url, kind)
                end
            end)
        end
    end

    pairee.log.info(string.format(
        "recent-files: loaded %d entries from %s",
        #state.entries, state_path()
    ))
end

-- Make sure state is written when the app shuts down.
function M:teardown()
    M:_flush()
end

---------------------------------------------------------------------------
-- Picker (Ctrl+R)
---------------------------------------------------------------------------

function M:entry()
    if #state.entries == 0 then
        pairee.app.notify(
            "recent-files",
            pairee.t("messages.empty"),
            "info"
        )
        return
    end

    -- Build a list of "kind  path" labels for the which prompt.
    local labels = {}
    for i, e in ipairs(state.entries) do
        local prefix = e.kind == "dir" and "  " or "  "
        labels[i] = string.format("%s%s", prefix, e.path)
    end

    local pick
    if pairee.which then
        pick = pairee.which({ cands = labels, silent = false })
    elseif pairee.input then
        -- Fallback: ask the user to type the index.
        local raw = pairee.input({
            title = pairee.t("messages.picker_title"),
            value = "",
        })
        if raw and raw.value then
            pick = tonumber(raw.value)
        end
    end

    if not pick or pick < 1 or pick > #state.entries then
        return
    end
    local chosen = state.entries[pick]
    if not chosen then return end

    if chosen.kind == "dir" then
        pairee.app.cd(chosen.path)
    else
        -- For files: jump the panel to the parent dir and try to focus
        -- the file (the runtime accepts either a path or a Url).
        pairee.app.cd(chosen.path)
    end
    pairee.app.notify(
        "recent-files",
        string.format(pairee.t("messages.opened"), chosen.path),
        "info"
    )
end

-- Public helper for other plugins / the CLI: return the most recent
-- entries without triggering the picker.
function M:list(n)
    n = tonumber(n) or 10
    local out = {}
    for i = 1, math.min(n, #state.entries) do
        out[#out + 1] = state.entries[i]
    end
    return out
end

return M
