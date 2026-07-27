# 📖 Reference: Plugin Lua API (`pairee.*`)

> **Quadrant: REFERENCE** — *information-oriented.*

This page is the full reference for the Lua API exposed to Pairee
plugins. For the user-side workflow (install, trust, update), see
[`28_howto_plugins`](28_howto_plugins.md). For the sandbox and trust
model, see [`51_explanation_plugin_sandbox`](51_explanation_plugin_sandbox.md).

> **Audience**: plugin authors. A plugin's `main.lua` is the single
> entry point; every API described here is reachable as
> `pairee.<name>` (or, for the legacy family, `pairee.app.<name>`).

---

## 1. Top-level namespace

| Entry | Purpose | Stability |
| --- | --- | --- |
| `pairee.app` | Legacy application surface. | Stable. `confirm` / `input` log a deprecation warning. |
| `pairee.emit(action, args)` | Dispatch any registered action by name. | Stable. |
| `pairee.confirm({...})` | Open a real Y/N confirm dialog. | Stable. |
| `pairee.input({...})` | Open a real input dialog. | Stable. |
| `pairee.which({cands, silent})` | Prompt for one of several keys. | Stable. |
| `pairee.notify({...})` | Show a structured notification. | Stable. |
| `pairee.file_cache({file, skip})` | Stable cache path for `(file, skip)`. | Stable. |
| `pairee.utils.*` | OS, time, hash helpers. | Stable. |
| `pairee.fs.*` | Filesystem operations. | Stable. |
| `pairee.ui.*` | Widget constructors (`Paragraph`, `Gauge`, `List`, `Table`, `Span`, `Line`). | Stable; richer userdata widgets land in a later release. |
| `pairee.ps.sub / pub / unsub` | Local pub/sub. | Stable. |
| `pairee.log.*` | Log a message at a level. | Stable. |
| `pairee.sync(fn)` | Snapshot-bridge into the main thread. | Stable. |
| `pairee.settings.*` | Read access to the plugin's resolved settings. | Stable. |
| `pairee.t(key, vars)` | Localised string lookup with variable interpolation. | Stable. |
| `pairee._secure_mode` | `true` when Secure Mode is active. | Stable. |

---

## 2. `pairee.emit(action, args)`

The single, unified entry point for triggering any registered action.

```lua
pairee.emit("cd", "/tmp")                  -- string arg
pairee.emit("cd", { path = "/tmp" })       -- table arg
pairee.emit("set_focus", "left")           -- alias: "focus" also works
pairee.emit("select", { url = f.url, state = true })
```

`args` is converted to JSON (Lua table → JSON object, integer-indexed
table → JSON array, scalar → scalar) and forwarded to the main thread.
The dispatcher runs the action synchronously on the main thread.

| Action | Args | Effect |
| --- | --- | --- |
| `"cd"` | `string` or `{path = string}` | Navigate the active panel. |
| `"set_focus"` / `"focus"` | `string` or `{side = string}` | Switch focus to `"left"` or `"right"`. |
| Any other action | per resolver | Logs a warning if not handled. |

Fire-and-forget: `pairee.emit` does not return a result.

---

## 3. `pairee.confirm({pos, title, body})`

```lua
local ok = pairee.confirm({
    pos   = { "center", w = 50, h = 10 },
    title = "Overwrite file?",
    body  = "The destination already exists.",
})
if not ok then return end
```

Returns `true` on accept, `false` on cancel.

---

## 4. `pairee.input({pos, title, value, obscure, realtime, debounce})`

```lua
local r = pairee.input({
    pos      = { "top-center", w = 60, h = 3 },
    title    = "New folder name",
    value    = "",
    obscure  = false,
    realtime = false,
    debounce = 0.3,
})
if r then print("user typed:", r.value, "event:", r.event) end
```

| Field | Default | Effect |
| --- | --- | --- |
| `pos` | required | `{"center" | "top-center", w = N, h = N}`. |
| `title` | `""` | Popup title. |
| `value` | `""` | Initial text. |
| `obscure` | `false` | Mask the input (passwords). |
| `realtime` | `false` | Fire on every keystroke. |
| `debounce` | `0` | If `realtime`, seconds to wait before firing. |

Returns `nil` on cancel, or `{ value, event }` on submit. `event`:

| Value | Meaning |
| --- | --- |
| `0` | unknown / channel closed |
| `1` | submitted (Enter) |
| `2` | cancelled (Esc) |
| `3` | typed (realtime only) |

---

## 5. `pairee.which({cands, silent})`

Prompts the user to press one of several keys. Returns the 1-based
index of the chosen candidate, or `nil` if cancelled.

```lua
local idx = pairee.which({
    silent = false,
    cands = {
        { on = "a",                 desc = "press a" },
        { on = "<C-c>",             desc = "cancel" },
        { on = { "j", "<Down>" },   desc = "down" },
    },
})
```

`on` is a single key string or a list of equivalent keys. `desc` is
optional human-readable text.

> The key notation follows the same parser as the resolver
> (`<C-c>` = Ctrl+C, `<Down>` = arrow, etc.).

---

## 6. `pairee.notify({title, content, level, timeout})`

```lua
pairee.notify({
    title   = "Hello",
    content = "World",
    level   = "info",    -- "info" | "warn" | "error", default "info"
    timeout = 2.5,       -- auto-dismiss in seconds
})
```

The toast appears at the top of the screen and disappears after
`timeout` (or stays until the user clicks it, if `timeout = 0`).

---

## 7. `pairee.file_cache({file, skip})`

Returns an absolute path under `<Pairee cache>/preview_cache/` that is
unique to the `(file, skip)` pair. Use it to cache expensive
previewer output (image conversions, OCR results, etc.).

```lua
local cache = pairee.file_cache({ file = job.file.url, skip = job.skip })
if cache then
    local f = io.open(cache, "r")
    if f then f:close() else
        -- generate the cache file
    end
end
```

The cache directory is created lazily on the first call.
`pairee.file_cache` returns `nil` if the directory cannot be created.

---

## 8. `pairee.utils.*`

```lua
local os   = pairee.utils.target_os()      -- "linux" | "macos" | "windows" | ...
local fam  = pairee.utils.target_family()  -- "unix"  | "windows" | "wasm"
local now  = pairee.utils.time()           -- float seconds since UNIX epoch
local hash = pairee.utils.hash("payload")  -- 16-char hex string
```

> `hash` is non-cryptographic. It is stable across runs of the same
> Pairee binary but not portable across architectures or even library
> versions — do not use it for security-sensitive comparisons.

---

## 9. `pairee.fs.*`

| Function | Effect |
| --- | --- |
| `read(path)` → `string` or `nil, err` | Read a file as text. |
| `write(path, data)` | Write `data` to `path`. |
| `exists(path)` → `bool` | Path exists. |
| `stat(path)` → table or `nil, err` | Same fields as `stat(2)`. |
| `list(path)` → array of file entries | List a directory. |
| `spawn(cmd, args, opts)` → handle | Spawn a child process (sandbox-restricted). |
| `spawn_copy_task(src, dst, opts)` → task | Queue an async copy. |

`opts` accepts `overwrite = "ask" | "overwrite" | "skip" | "append"`,
`preserve = true | false`, and `filter = "glob"`.

> `spawn` is **only available** to trusted plugins. See
> [`51_explanation_plugin_sandbox`](51_explanation_plugin_sandbox.md).

---

## 10. `pairee.ui.*`

Widget constructors. Return a userdata handle that you can compose
into lines and paragraphs.

| Constructor | Effect |
| --- | --- |
| `Paragraph` | Multi-line text widget. |
| `Gauge` | Horizontal progress bar. |
| `List` | Vertical list of items. |
| `Table` | Grid with headers. |
| `Span`, `Line` | Inline text fragments. |

Richer widgets (Button, Input, Checkbox) are scheduled for a later
release.

---

## 11. `pairee.log.*`

```lua
pairee.log.debug("loading file", { path = path })
pairee.log.info("done")
pairee.log.warn("retrying")
pairee.log.error("failed", { code = err })
```

Messages go to the in-app log buffer and `app.log` in the cache
directory.

---

## 12. `pairee.settings.*`

Read-only access to the plugin's resolved settings (from
`manifest.toml` and any user-level overrides).

```lua
local level = pairee.settings.get("log_level", "info")
```

Returns `nil` if the key is missing.

---

## 13. `pairee.t(key, vars)`

Localised string lookup with variable interpolation.

```lua
local s = pairee.t("greeting", { name = "Lara" })
-- "greeting" = "Hello, {name}!"
```

The TOML file is `<plugin>/lang/<lang>.toml`. Pairee falls back to
`en` if the active language is missing.

---

## 14. Legacy `pairee.app.*`

The legacy family is still supported; new code should use the
structured forms above.

| Old | New |
| --- | --- |
| `pairee.app.cd(path)` | `pairee.emit("cd", path)` or `pairee.emit("cd", { path = path })` |
| `pairee.app.set_focus(side)` | `pairee.emit("set_focus", side)` or `pairee.emit("focus", side)` |
| `pairee.app.confirm(title, msg)` | `pairee.confirm({pos=…, title=…, body=msg})` (logs deprecation) |
| `pairee.app.input(title, default)` | `pairee.input({pos=…, title=…, value=default, obscure=…, realtime=…, debounce=…})` (logs deprecation) |
| `pairee.app.notify(title, msg, level)` | `pairee.notify({title=…, content=msg, level=…, timeout=…})` |

---

## 15. Cross-platform notes

- `pairee.utils.target_os()` returns the compile-time OS string from
  `std::env::consts::OS`. Use it to gate OS-specific code paths.
- `pairee.utils.target_family()` returns `"unix"`, `"windows"`, or
  `"wasm"`. Prefer it for portability checks.
- File paths in Pairee are always `std::path::Path`; never hardcode
  `/` or `\`. Plugins receive paths as strings and should use the
  platform-appropriate separator (Lua's `package.config:sub(1,1)` gives
  it on the running platform).

---

## Where to go next

- Sandbox and trust model: [`51_explanation_plugin_sandbox`](51_explanation_plugin_sandbox.md)
- Plugin authoring: https://github.com/FittyAr/Pairee/blob/master/docs/plugin-dev-guide.md
- Plugin architecture: https://github.com/FittyAr/Pairee/blob/master/docs/technical/plugin-system-design.md
