# Pairee Plugin Lua API Reference

This document describes the Lua API surface exposed to Pairee plugins, including the new bindings introduced in the M0 plugin system evolution. For full architecture and design rationale, see [`docs/technical/plugin-system-design.md`](../technical/plugin-system-design.md) and [`docs/technical/plugin-roadmap.md`](../technical/plugin-roadmap.md). For a step-by-step guide to writing a plugin, see [`docs/plugin-dev-guide.md`](../plugin-dev-guide.md).

---

## 1. Global namespace `pairee`

Inside a plugin's `main.lua`, the table `pairee` is the only public entry point. Every function in this document is reachable as `pairee.<name>` (or, for the legacy `app.*` family, `pairee.app.<name>`).

| Entry | Purpose | Status |
|---|---|---|
| `pairee.app` | Legacy application surface (cwd, cd, focus, set_focus, notify, confirm, input, hovered) | Stable; `confirm`/`input` emit a deprecation warning — use the top-level forms |
| `pairee.emit(action, args)` | Dispatch any registered action by name | New in M0 |
| `pairee.confirm({pos, title, body})` | Open a real confirm dialog (Y/N) | New in M0 (popup UI ships in M1) |
| `pairee.input({pos, title, value, obscure, realtime, debounce})` | Open a real input dialog | New in M0 (popup UI ships in M1) |
| `pairee.which({cands, silent})` | Prompt the user to press one of several candidate keys | New in M0 (popup UI ships in M1) |
| `pairee.notify({title, content, level, timeout})` | Show a structured notification | New in M0 |
| `pairee.file_cache({file, skip})` | Get a stable cache path for a `(file, skip)` pair | New in M0 |
| `pairee.utils.target_os()` | Return `"linux"` / `"macos"` / `"windows"` / ... | New in M0 |
| `pairee.utils.target_family()` | Return `"unix"` / `"windows"` / `"wasm"` | New in M0 |
| `pairee.utils.time()` | Return the current UNIX epoch in seconds (float) | New in M0 |
| `pairee.utils.hash(str)` | Return a stable 64-bit hash of `str` as a hex string | New in M0 |
| `pairee.fs.*` | Filesystem operations (`read`, `write`, `exists`, `stat`, `list`, `spawn`, `spawn_copy_task`) | Stable |
| `pairee.ui.*` | Widget constructors (`Paragraph`, `Gauge`, `List`, `Table`, `Span`, `Line`) | Stable; richer userdata widgets land in M4 |
| `pairee.ps.sub / pub / unsub` | Local pub/sub | Stable |
| `pairee.log.*` | Log a message at the given level | Stable |
| `pairee.sync(fn)` | Snapshot-bridge into the main thread state | Stable; full sync/async dual path lands in M3 |
| `pairee.settings.*` | Read access to the plugin's resolved settings | Stable |
| `pairee.t(key, vars)` | Localised string lookup with variable interpolation | Stable |
| `pairee._secure_mode` | Boolean: `true` when global Secure Mode is active | Stable |

---

## 2. `pairee.emit(action, args)` — action dispatch

`pairee.emit` is the single, unified entry point for triggering any action that is registered in the application's key-binding resolver.

```lua
pairee.emit("cd", "/tmp")                        -- string arg
pairee.emit("cd", { path = "/tmp" })             -- table arg
pairee.emit("set_focus", "left")                 -- alias: "focus" also works
pairee.emit("select", { url = f.url, state = true })
```

`args` is converted to a JSON value (Lua table → JSON object, integer-indexed Lua table → JSON array, scalar → JSON scalar) and forwarded to the main thread. The dispatcher runs the action synchronously on the main thread.

| Status | Action name | Args | Effect |
|---|---|---|---|
| M0 | `"cd"` | `string` or `{path = string}` | Navigate the active panel to the given path |
| M0 | `"set_focus"` / `"focus"` | `string` or `{side = string}` | Switch focus to `"left"` or `"right"` |
| Future | *any other action* | per resolver | Logs a warning in M0; will dispatch through the resolver in a later phase |

The dispatcher is fire-and-forget; `pairee.emit` does not return a result.

---

## 3. `pairee.confirm({pos, title, body})` and `pairee.input({pos, title, value, obscure, realtime, debounce})`

These are the new structured dialog APIs. They replace the legacy `pairee.app.confirm(title, msg)` and `pairee.app.input(title, default)` stubs.

```lua
-- Confirm: returns true if the user accepts, false if they cancel.
local ok = pairee.confirm({
    pos   = { "center", w = 50, h = 10 },
    title = "Overwrite file?",
    body  = "The destination file already exists.",
})
if not ok then return end

-- Input: returns a table { value, event } on submit, or nil on cancel.
local result = pairee.input({
    pos      = { "top-center", w = 60, h = 3 },
    title    = "New folder name",
    value    = "",
    obscure  = false,
    realtime = false,
    debounce = 0.3,
})
if result then
    print("user typed:", result.value, "event:", result.event)
end
```

`event` is an integer tag:

| Value | Meaning |
|---|---|
| 0 | unknown / channel closed (default) |
| 1 | submitted (Enter) |
| 2 | cancelled (Esc) |
| 3 | typed (realtime only) |

**M0 note**: the dispatcher routes the request, but the actual TUI popup wiring ships in M1. In M0 both dialogs return placeholder values (`false` for confirm, `submitted` with the default value for input) so plugins that migrate early still get a deterministic answer.

### 3.1 Legacy `pairee.app.confirm(title, msg)` and `pairee.app.input(title, default)`

These still work but log a deprecation warning. Migrate to the structured forms above.

---

## 4. `pairee.which({cands, silent})` — key-prompt

Prompts the user to press one of the candidate keys, returns the 1-based index of the selected candidate (or `nil` if the user cancels).

```lua
local idx = pairee.which({
    silent = false,
    cands = {
        { on = "a",                 desc = "press a" },
        { on = "<C-c>",             desc = "cancel" },
        { on = { "j", "<Down>" },   desc = "down" },
    },
})
if idx == 1 then
    -- user pressed a
end
```

`on` may be a single key string or a list of equivalent keys. `desc` is an optional human-readable description shown next to the candidate.

**M0 note**: the actual TUI popup wiring ships in M1. In M0 the dispatcher returns `nil` (cancel) so plugins that migrate early get a deterministic placeholder.

---

## 5. `pairee.notify({title, content, level, timeout})` — structured notification

```lua
pairee.notify({
    title   = "Hello",
    content = "World",
    level   = "warn",        -- "info" | "warn" | "error", default "info"
    timeout = 2.5,          -- auto-dismiss in seconds (M0: logged but not enforced)
})
```

The legacy `pairee.app.notify(title, msg, level)` still works. New plugins should use the structured form.

---

## 6. `pairee.file_cache({file, skip})` — stable cache path

Returns an absolute path under `<Pairee cache>/preview_cache/` that is unique to the `(file, skip)` pair. Use this to cache expensive previewer output (image conversions, OCR results, etc.) across invocations.

```lua
local cache = pairee.file_cache({ file = job.file.url, skip = job.skip })
if cache then
    local f = io.open(cache, "r")
    if f then f:close() else -- generate the cache file end
end
```

The cache directory is created lazily on the first call. `pairee.file_cache` returns `nil` if the cache directory cannot be created.

---

## 7. `pairee.utils.*` — basic utilities

```lua
local os   = pairee.utils.target_os()      -- "linux" | "macos" | "windows" | ...
local fam  = pairee.utils.target_family()  -- "unix"  | "windows" | "wasm"
local now  = pairee.utils.time()           -- float seconds since UNIX epoch
local hash = pairee.utils.hash("payload")  -- 16-char hex string
```

`hash` is non-cryptographic (uses the standard library's `DefaultHasher`, which is xxhash-based). It is stable across runs of the same Pairee binary but is not portable across architectures or even library versions — do not use it for security-sensitive comparisons.

---

## 8. Sandbox and secure mode

Plugins run in a Lua sandbox that:

- Exposes only `base`, `table`, `string`, `utf8`, and `math` (in untrusted mode).
- Removes `load`, `loadstring`, `dofile`, and `loadfile` (in untrusted mode).
- Replaces `require` with a path-bounded loader limited to the plugin's own directory.
- Blocks `pairee.fs.spawn` and any `io`/`os`/`package` access (in untrusted mode).

When Secure Mode is active in the user's `pairee.toml`:

- `pairee.fs.spawn` is additionally blacklisted against a 27-command list (network tools, shells, script runtimes).
- File-system operations are restricted to the active workspace + the user's config and cache directories.

See the `[sandbox]` section in `docs/plugin-dev-guide.md` for the full matrix.

---

## 9. Cross-platform notes

- `pairee.utils.target_os()` returns the compile-time OS string from `std::env::consts::OS`. Use it to gate OS-specific code paths.
- `pairee.utils.target_family()` returns `"unix"`, `"windows"`, or `"wasm"`. Prefer it over `target_os` for portability checks.
- File paths in Pairee are always `std::path::Path`; never hardcode `/` or `\`. Plugins receive paths as strings and should use the platform-appropriate separator (Lua's `package.config:sub(1,1)` gives it on the running platform).

---

## 10. Migration cheatsheet

| Old (M0 and earlier) | New (M0+) | Notes |
|---|---|---|
| `pairee.app.cd(path)` | `pairee.emit("cd", path)` or `pairee.emit("cd", { path = path })` | Old form still works |
| `pairee.app.set_focus(side)` | `pairee.emit("set_focus", side)` or `pairee.emit("focus", side)` | Old form still works |
| `pairee.app.confirm(title, msg)` | `pairee.confirm({pos=..., title=title, body=msg})` | Old form logs deprecation, returns `true` |
| `pairee.app.input(title, default)` | `pairee.input({pos=..., title=title, value=default, obscure=..., realtime=..., debounce=...})` | Old form logs deprecation, returns `default` |
| `pairee.app.notify(title, msg, level)` | `pairee.notify({title=title, content=msg, level=level, timeout=...})` | Old form still works |
| (no equivalent) | `pairee.which({cands=..., silent=...})` | M0 returns `nil` (cancel); M1 wires the popup |
| (no equivalent) | `pairee.file_cache({file=..., skip=...})` | M0 fully functional |
| (no equivalent) | `pairee.utils.target_os / target_family / time / hash` | M0 fully functional |

For the full gap analysis and roadmap to M1–M5, see [`docs/technical/plugin-roadmap.md`](../technical/plugin-roadmap.md).
