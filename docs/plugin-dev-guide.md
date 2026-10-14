# Pairee Plugin Developer Guide

> **This guide explains how to write, test, and submit a Lua plugin for Pairee.**

---

## Table of Contents

1. [Overview](#1-overview)
2. [Plugin Types](#2-plugin-types)
3. [Your First Plugin — Hello World](#3-your-first-plugin--hello-world)
4. [Plugin File Structure](#4-plugin-file-structure)
5. [API Reference Summary](#5-api-reference-summary)
6. [Commands and Keyboard Shortcuts](#6-commands-and-keyboard-shortcuts)
7. [Writing a Previewer Plugin](#7-writing-a-previewer-plugin)
8. [Writing a Hook Plugin](#8-writing-a-hook-plugin)
9. [Writing a Command Plugin](#9-writing-a-command-plugin)
10. [State Synchronization with `pairee.sync`](#10-state-synchronization-with-paireesync)
11. [Pub/Sub Events with `pairee.ps`](#11-pubsub-events-with-paireeps)
12. [Debugging and Error Handling](#12-debugging-and-error-handling)
13. [Trusted Mode and Secure Mode Protection](#13-trusted-mode-and-secure-mode-protection)
14. [Testing Your Plugin Locally](#14-testing-your-plugin-locally)
15. [Developer Mode Tools & PR Submission TUI](#15-developer-mode-tools--pr-submission-tui)
16. [Registry Submission Flow](#16-registry-submission-flow)
17. [Manifest Reference](#17-manifest-reference)
18. [Best Practices & Conventions](#18-best-practices--conventions)

---

## 1. Overview

Pairee plugins are **Lua modules** that extend the file manager with:
- **File previewers** — render custom content in the preview pane for specific file types.
- **Lifecycle hooks** — react to navigation events (directory changes, file hover, key presses).
- **Functional commands** — execute actions invoked via keybindings or the user menu.

Plugins are stored in your configuration folder and declared in `pairee.toml`. Once loaded, they run asynchronously on background threads without blocking terminal UI rendering.

---

## 2. Plugin Types

| Type | Lua Methods | Invoked When |
|------|-------------|--------------|
| **Previewer** | `peek(job)`, `seek(job)` | The Quick View shows a file (`peek`); PgUp / PgDn on the plugin's preview (`seek`) |
| **Preloader** | `preload(job)` | The Quick View shows a file, and for the next three files of the panel |
| **Hook** | `setup(opts)`, subscriptions via `pairee.ps.sub` | A lifecycle event fires (e.g. `on_cd`) |
| **Command** | `entry(args)` | A keybinding or menu item calls the plugin by name |
| **Mixed** | Combination of the above | Multiple roles |

---

## 3. Your First Plugin — Hello World

Create the folder `~/.config/pairee/plugins/hello/`.

Create the file `~/.config/pairee/plugins/hello/main.lua`:

```lua
local M = {}

function M:entry()
    pairee.app.notify("Hello!", "This is my first Pairee plugin.", "info")
end

return M
```

Create the manifest file `~/.config/pairee/plugins/hello/manifest.toml`:

```toml
name = "hello"
version = "1.0.0"
description = "Simple Hello World test plugin"
author = "your-handle"
license = "MIT"
type = "command"
min_pairee = "0.7.0"   # semver; pre-releases like "0.10.0-beta.1" work, an unreadable value refuses to load
```

Register it in `pairee.toml`:

```toml
[plugins.hello]
name    = "hello"
trusted = false
```

Launch Pairee, open the command input (or bind it to a key), and run its command from the command palette. A notification will appear.

---

## 4. Plugin File Structure

A plugin is a folder containing a main Lua script, manifest, and optional submodules or resources:

```text
~/.config/pairee/plugins/
└── my-plugin/
    ├── main.lua              # Required — plugin entry point
    ├── manifest.toml         # Required — metadata and commands
    ├── utils.lua             # Optional submodule
    └── locale/               # Optional localization files
        ├── en.toml
        └── es.toml
```

`main.lua` **must** return a Lua table `M` containing the lifecycle/trigger functions.

---

## 5. API Reference Summary

### `pairee.app` — Application Control

```lua
pairee._lua_api_version                 -- "1.3.0" (Lua surface semver)
pairee.app.cwd()                        -- string: current directory
pairee.app.cd(path)                     -- navigate to path
pairee.app.focus()                      -- "left" | "right"
pairee.app.set_focus(side)              -- switch panel
pairee.app.notify(title, msg, level)   -- show popup ("info","warn","error")
pairee.app.confirm(title, msg)          -- boolean (deprecated; opens real TUI)
pairee.app.input(title, default)        -- string (deprecated; opens real TUI)
pairee.app.hovered()                    -- table snapshot of hovered file
```

### `pairee.confirm` / `pairee.input` / `pairee.which` — Interactive dialogs

```lua
pairee.confirm({ pos, title, body })    -- boolean (Y/Enter or N/Esc)
pairee.input({ pos, title, value, obscure, realtime, debounce })
                                        -- { value, event } or nil
pairee.which({ cands, silent })         -- 1-based index or nil
```

### `pairee.cx` — Panel context (inside `pairee.sync`)

```lua
pairee.sync(function()
    local cwd = pairee.cx.active.cwd
    local hovered = pairee.cx.active.hovered  -- File userdata or nil
    if hovered then
        print(hovered.name, hovered.size, hovered.mime, hovered.is_hidden)
    end
    local selected = pairee.cx.active.selected -- File[]
end)
```

### `pairee.fs` — Filesystem & Processes

```lua
pairee.fs.read(path)                    -- string: file contents
pairee.fs.write(path, data)            -- write data to file
pairee.fs.exists(path)                 -- boolean
pairee.fs.stat(path)                   -- File userdata (or nil)
pairee.fs.list(path)                   -- File[]: directory listing
pairee.fs.data_dir()                   -- string: private writable dir for this plugin
pairee.fs.read_dir(path)               -- File[] (same as list)
pairee.fs.file(path)                   -- File userdata
pairee.fs.mkdir("dir"|"dir_all", path)
pairee.fs.remove("file"|"dir"|"dir_all"|"dir_clean", path)
pairee.fs.rename(from, to)
pairee.fs.copy(from, to)               -- bytes copied
pairee.fs.spawn(cmd, args)             -- Output: {stdout, stderr, status}
pairee.fs.spawn_copy_task(from, to)    -- background copy with progress popup
```

### `pairee.Command` — process builder (trusted plugins)

```lua
local out = pairee.Command("rg")
    :arg("-n")
    :arg(pattern)
    :cwd(pairee.cx.active.cwd)
    :stdout(pairee.Command.PIPED)
    :stderr(pairee.Command.PIPED)
    :output()                       -- Output { stdout, stderr, status }

local child = pairee.Command("cat")
    :stdin(pairee.Command.PIPED)
    :stdout(pairee.Command.PIPED)
    :spawn()
child:write_all("hello\n")
child:close_stdin()
local streamed = child:wait_with_output()
```

In Secure Mode (13.3) `:env(name, value)` only accepts locale, terminal and
colour variables (`LANG`, `LANGUAGE`, `LC_*`, `TERM`, `COLORTERM`, `NO_COLOR`,
`CLICOLOR`, `CLICOLOR_FORCE`, `FORCE_COLOR`, `TZ`, `COLUMNS`, `LINES`); any
other name raises a security violation. `:cwd(path)` must name an existing
folder inside the plugin's `pairee.fs` read jail.

### `pairee.ui` — Widgets

Widgets are userdata built with chained calls; `peek` returns one (see
section 7). Every widget and text value takes the style methods `:style(s)`,
`:fg(color)`, `:bg(color)`, `:bold()`, `:dim()`, `:italic()`, `:underline()`,
`:blink()`, `:reverse()`, `:hidden()`, `:crossed()` and `:reset()` (colors are
theme names such as `"red"`, `"darkgray"` or `"#rrggbb"`).

```lua
local ui = pairee.ui
ui.Style():fg("cyan"):bold()                     -- reusable style
ui.Span("text", "red")                           -- styled text (style or color)
ui.Line({ "plain ", ui.Span("bold"):bold() }):align(ui.Align.CENTER)
ui.Text("line 1\nline 2")                        -- also a Line, Span or list of them
ui.Text.parse("\27[31mred\27[0m")                -- keeps ANSI colors
ui.Paragraph(text):wrap(ui.Wrap.YES):scroll(2):block(ui.Border(ui.Edge.ALL))
ui.List({ "a", ui.Span("b"):fg("yellow") })
ui.Table({ ui.Row({ "a.txt", "12" }), { "b.txt", "7" } })
    :header({ "Name", "Size" })
    :widths({ ui.Constraint.Length(20), ui.Constraint.Fill(1) })
ui.Gauge():percent(40):label("40 %"):gauge_style(ui.Style():fg("green"))
ui.Border(ui.Edge.TOP + ui.Edge.BOTTOM):type(ui.Border.ROUNDED):title("Info")
ui.Layout():direction(ui.Layout.HORIZONTAL):margin(1)
    :constraints({ ui.Constraint.Percentage(30), ui.Constraint.Fill(1) })
    :children({ left_widget, right_widget })
```

Constants: `ui.Align.{LEFT,CENTER,RIGHT}`, `ui.Wrap.{NO,YES,TRIM}`,
`ui.Edge.{NONE,TOP,RIGHT,BOTTOM,LEFT,ALL}`,
`ui.Border.{PLAIN,ROUNDED,DOUBLE,THICK}`, `ui.Layout.{HORIZONTAL,VERTICAL}`;
`ui.Constraint.{Min,Max,Length,Percentage,Ratio,Fill}`. The layout is
declarative: Pairee splits the preview area when it draws, so there is no
`split(rect)`. The 1.0 forms (`ui.Gauge(ratio, label)`, `ui.Table(headers,
rows)`, `ui.Span(text, color)`, tables with a `type` field) still work. A
widget may hold at most 100 000 nodes and 8 MiB of text.

### Typed values — `Path`, `Url`, `Cha`, `Error`

```lua
local p = pairee.Path("docs/guide.md")   -- or pairee.Path.os(...)
p.name, p.stem, p.ext, p.parent, p.is_absolute, p.has_root
p:join("x"), p:starts_with("docs"), p:ends_with("guide.md"), p:strip_prefix("docs")

local u = pairee.Url("sftp://me@host//srv/www/index.html")
u.scheme, u.domain, u.path, u.name, u.parent, u.is_regular   -- "sftp", "me@host", ...
u:join("css"), u:starts_with("sftp://me@host/srv")           -- compares scheme and host too

local cha, err = pairee.fs.cha(path, follow)   -- Cha, nil  |  nil, Error
cha.is_dir, cha.is_link, cha.is_orphan, cha.is_hidden, cha.is_exec, cha.len
cha.mtime, cha.atime, cha.btime, cha.mode, cha.uid, cha.gid, cha.nlink
cha:perm()                                    -- "drwxr-xr-x" (nil on Windows)
cha:hash()                                    -- hex digest of size and times
file.cha                                      -- the same for a File

pairee.Error.custom("message")                -- err.kind, err.code, err.message
```

`Path` and `Url` never touch the disk. Every `pairee.fs` function also accepts
a `Path` or a local `Url`. The newer APIs (`fs.cha`, `clipboard`, `state`,
`image`, `preview_code`, `preview_widget`) return `value, nil` on success and
`nil` (or `false`), `Error` on failure; `err.kind` is an `std::io` kind such
as `"NotFound"` or `"PermissionDenied"`. Older APIs keep throwing.

### `pairee.clipboard`, `pairee.state`, identity

```lua
pairee.clipboard("text")             -- true | false, Error  (every plugin)
local text, err = pairee.clipboard() -- trusted plugins outside Secure Mode only

pairee.state.set("count", 3)         -- true | false, Error  (nil deletes)
pairee.state.get("count")            -- 3, kept between calls and sessions
pairee.state.all(), pairee.state.clear()

pairee.utils.user_name(), pairee.utils.host_name()
pairee.utils.uid(), pairee.utils.gid(), pairee.utils.group_name()  -- nil on Windows
```

`pairee.state` keeps JSON-compatible values (scalars, lists, string-keyed
tables) in `state.json` of the plugin's data directory, at most 256 KiB; a
change that would go over is refused. Text put on the clipboard loses control
characters other than tab and line feed and is limited to 1 MiB.

### `pairee.rt`, `pairee.th`

```lua
pairee.rt.version, pairee.rt.api, pairee.rt.os, pairee.rt.family
pairee.rt.plugin.name, .dir, .data_dir, .trusted, .secure_mode
pairee.rt.mgr.show_hidden, .sort_by, .sort_reverse
pairee.rt.preview.image, .max_bytes
pairee.rt.term.size()                 -- cols, rows

pairee.th.name                        -- active theme
pairee.th.header_fg                   -- ui.Style of a theme color (`*_bg` keys set the background)
```

### `pairee.image` and preview helpers

```lua
pairee.image.show(job.file)            -- image widget (drawn like the Quick View's images)
pairee.image.info(path)                -- { w, h, format, color }
pairee.image.precache(src, dst, { max_width = 512, max_height = 512 })  -- PNG thumbnail
pairee.preview_code({ file = job.file, skip = job.skip, lines = job.area.h })
pairee.preview_widget({ file = path, skip = 0 }, widget)  -- show a widget later
```

`image.show`, `image.info` and `preview_code` accept the file being previewed
(the `peek` / `preload` / `seek` job's file) even when it is outside the
plugin's `pairee.fs` jail; `precache` needs a source the jail allows.
`preview_code` shows the text with line numbers, highlighted like the built-in
viewer when syntax highlighting is on and a grammar knows the file.

### `pairee.ps` — Pub/Sub

```lua
pairee.ps.sub(event, fn)               -- subscribe to event
pairee.ps.pub(event, data)             -- publish event
pairee.ps.unsub(event)                 -- unsubscribe
```

### `pairee.log` — Logging API

```lua
pairee.log.info(msg)                   -- Log info level message
pairee.log.warn(msg)                   -- Log warning level message
pairee.log.error(msg)                  -- Log error level message
pairee.log.debug(msg)                  -- Log debug level message
```

---

## 6. Commands and Keyboard Shortcuts

A plugin declares its commands in `manifest.toml`, each with the keys it
suggests. Pairee adds them to the same keymap as its built-in actions, so they
work with key sequences and the `<leader>` key, appear in the command palette
and in the keyboard-shortcuts list, and can be rebound by the user.

```toml
# In manifest.toml
[[commands]]
id    = "toggle"               # keymap id: plugin.<plugin name>.toggle
title = "Toggle blame"         # shown in the palette and the shortcuts list

[commands.keys]                # per preset; "default" for every other one
default  = "Ctrl+Alt+b"
neovim   = "<leader>gb"
yazi     = "g b"
standard = "Alt+Shift+B"
```

Pressing the key calls `entry(args)`; `args.command` (and `args[1]`) is the
command id, so one `entry` can serve several commands:

```lua
function M:entry(args)
    if args.command == "toggle" then
        -- ...
    end
end
```

**Keys are suggestions.** A key the active preset already uses stays unbound
for the plugin (Pairee keeps the user's keys working); when two plugins
suggest the same key, the plugin whose name sorts first gets it. Unbound
commands remain in the palette. Users rebind any command in
`keybindings.toml`:

```toml
[overrides.all]
"plugin.git-blame.toggle" = "Ctrl+Alt+g"
```

To avoid conflicts, prefer `<leader>p…` in Neovim, `Alt+p` followed by a
letter in yazi, and `Ctrl+Alt+x` followed by a letter in the other presets.
`pairee developer lint` lists, per preset, the suggested keys that are taken
or invalid.

Key spellings: `Ctrl+h`, `ctrl-h` and `<C-h>` are the same chord; an uppercase
letter means Shift (`Ctrl+H`); a space separates the keys of a sequence
(`g b`).

**Reading the keymap from Lua.** `pairee.keymap.list()` returns every live
binding as `{ chord, id, label, origin }`; `pairee.keymap.chord_for(id)`
returns the first chord of an action (`"copy"`, `"plugin.<name>.<command>"`)
or `nil`. **Firing actions.** `pairee.emit(id)` runs a keymap action as if
its key were pressed (`pairee.emit("go_to_tab_2")`); `cd` and `focus` also
take arguments. What a plugin may fire depends on its trust:

* **Untrusted** plugins may only fire navigation, view, sort, selection and
  help actions (`move_down`, `go_parent`, `go_to_tab_<n>`, `sort_by_name`,
  `select_all`, `quick_view`, `help`, ...) and their **own** commands
  (`plugin.<this plugin>.<command>`). Anything that copies, moves, deletes,
  writes, runs commands, changes settings or connects is refused with an
  error dialog.
* **Trusted** plugins may fire any action. It runs exactly like the key
  press, so the usual confirmation dialogs (and the user's confirmation
  settings, such as `confirm_delete`) apply.
* `cd` (and `pairee.app.cd`) resolves a relative path against the active
  panel's folder and only accepts an existing local folder; it is refused
  on SSH and archive panels.
* Arguments are converted to JSON with limits: tables must be dense lists
  (`{ "a", "b" }`) or string-keyed maps, nested at most 32 levels and 100 000
  values in total. Sparse, mixed or cyclic tables raise a Lua error.

The pre-v2 table `[keybindings] "ctrl+h" = "toggle"` still works: each value
becomes a command with those keys as its default.

## 7. Writing a Previewer Plugin

Previewers implement `peek(job)` and optionally `preload(job)` and
`seek(job)`. When the Quick View shows a file, Pairee calls `preload` of every
plugin that has one, then `peek` of each previewer in name order; the first
widget returned replaces the built-in preview (return `nil` to decline). After
that, the next three files of the panel are preloaded in the background.

The `job` parameter provides:
- `job.file` — a `File` of the previewed file (`.path`, `.name`, `.mime`, `.size`, `.cha`, ...)
- `job.area` — the preview area in cells: `x`, `y`, `w`, `h` (also `width`, `height`)
- `job.skip` — lines scrolled
- `job.units` — (`seek` only) lines to scroll, negative upwards

`preload(job)` prepares something ahead (a thumbnail, a cache file) and returns
`complete[, err]`; a file version whose preload returned `true` is not
preloaded again. `seek(job)` returns the new `skip`; Pairee then calls `peek`
again with it. Without `seek`, PgUp / PgDn scroll the widget itself.

```lua
local M = {}

function M:peek(job)
    if not job.file.url:match("%.csv$") then
        return  -- not a CSV file, decline
    end

    local content = pairee.fs.read(tostring(job.file.url))
    local rows = {}
    for line in content:gmatch("[^\n]+") do
        local cols = {}
        for col in line:gmatch("[^,]+") do
            cols[#cols + 1] = col
        end
        rows[#rows + 1] = cols
    end

    local headers = table.remove(rows, 1)
    return pairee.ui.Table(rows):header(headers):style(pairee.th.panel_fg)
end

return M
```

An untrusted previewer cannot read files outside its folder with `pairee.fs`,
but it can still show the previewed file through the host:

```lua
function M:peek(job)
    if job.file.mime:match("^image/") then
        return pairee.image.show(job.file)
    end
    return pairee.preview_code({ file = job.file, skip = job.skip, lines = job.area.h })
end
```

---

## 8. Writing a Hook Plugin

Hook plugins subscribe to lifecycle events during their `setup(opts)` call.

**Available events:**

| Event | Payload | Fires When |
|-------|---------|------------|
| `on_cd` | `{ cwd: string }` | Active panel changes directory |
| `on_hover` | `{ entry: Entry }` | User moves cursor to a different file |
| `on_key` | `{ key: string }` | A key is pressed (before resolver) |
| `on_mouse` | `{ kind, button, x, y, target, side?, index? }` | A mouse click on the panels or the bars (not inside dialogs). `kind`: `click` / `double_click`; `button`: `left` / `right` / `middle`; `target`: `panel_row`, `panel`, `panel_title`, `panel_header`, `tab`, `menu`, `fkey`, `command_line`, `other`; `side`: `left` / `right`; `index`: 1-based entry of a `panel_row` |
| `on_focus` | `{ side: string }` | Panel focus switches |

```lua
local M = {}

function M:setup(opts)
    pairee.ps.sub("on_cd", function(payload)
        pairee.log.info("Navigated to: " .. payload.cwd)
        local result = pairee.fs.spawn("git", { "-C", payload.cwd, "status", "--short" })
        if result.status == 0 and result.stdout ~= "" then
            pairee.app.notify("Git Status", result.stdout, "info")
        end
    end)
end

return M
```

---

## 9. Writing a Command Plugin

Command plugins implement `entry(args)`. They run from their keys or from the command palette (see section 6).

```lua
local M = {}

function M:entry()
    local result = pairee.fs.spawn("fzf", { "--layout=reverse", "--height=40%" })
    if result.status == 0 and result.stdout ~= "" then
        local target = result.stdout:gsub("\n$", "")
        pairee.app.cd(target)
    end
end

return M
```

---

## 10. State Synchronization with `pairee.sync`

Plugin code runs asynchronously on background threads. Reading `AppState` directly from an async context is unsafe. Use `pairee.sync()` to receive a read-only snapshot:

```lua
-- Wrap the state-reading function at setup time
local read_state = pairee.sync(function()
    return {
        cwd   = pairee.app.cwd(),
        focus = pairee.app.focus(),
    }
end)

-- Call it later in async context
local state = read_state()
print(state.cwd)
```

**Rules:**
- `pairee.sync()` must be called at plugin load time (top-level or inside `setup`), not inside async callbacks.
- The returned callable is safe to call from any async context.

### Annotations

The leading comment block of `main.lua` may hold annotations:

```lua
--- @since 1.2.0
--- @sync entry
local M = {}
```

- `--- @since <version>` — the Lua API version the plugin needs; a Pairee with
  an older API refuses to load it (with that reason in the log) instead of
  failing later on a missing function.
- `--- @sync entry|peek|preload|seek` — before calling that method, Pairee
  refreshes `pairee.cx` (as `pairee.sync` does), so it reads the panels as they
  are at call time.

All plugin methods (`setup`, `entry`, `peek`, `preload`, `seek` and the
`pairee.ps` callbacks) run asynchronously, so they may wait for dialogs,
`pairee.sync`, `pairee.utils.sleep` or child processes.

---

## 11. Pub/Sub Events with `pairee.ps`

Plugins can communicate with each other through named pub/sub channels:

```lua
-- Publisher plugin
pairee.ps.pub("my-plugin:result", { path = "/some/path" })

-- Subscriber plugin
pairee.ps.sub("my-plugin:result", function(data)
    pairee.app.cd(data.path)
end)
```

Use namespaced event names (`your-plugin-name:event`) to avoid conflicts.

---

## 12. Debugging and Error Handling

Pairee provides rich logging and isolation features:

### 12.1 Logging API (`pairee.log`)
Use the logging functions to write information or debug markers directly to Pairee's log file located at `~/.cache/pairee/app.log`:
```lua
pairee.log.debug("Checking CSV structure...")
pairee.log.info("CSV Preview rendered successfully.")
pairee.log.error("Failed to parse row: " .. tostring(err))
```

### 12.2 Intercepting Runtime Errors
* Any runtime exception (such as syntax errors, accessing nil fields, or divide-by-zero) is captured by Pairee's Lua manager wrapper.
* These exceptions are intercepted and displayed as a visual error toast in the UI indicating the plugin name, the line number, and the specific error message.
* A full backtrace is written to `app.log` to help you troubleshoot.

### 12.3 CLI Debug Mode
To run Pairee in debug mode and trace your plugin execution live, launch the executable from a separate terminal with:
```bash
pairee --plugin-debug <plugin-name>
```
All calls to `pairee.log.*` and Lua runtime errors from that plugin will be output directly to stdout in the launching console.

---

## 13. Sandboxing, Trusted Mode, and Secure Mode Protection

To protect users against malicious or buggy third-party plugins, Pairee implements a strict, multi-layered security boundary.

### 13.1 The Lua Sandboxed VM (Untrusted Mode by Default)
By default, all plugins run in **Untrusted Mode** (`trusted = false`), which isolates them inside a secure sandbox VM:
* **Blocked Libraries:** The plugin cannot access Lua standard libraries `io`, `os`, `package`, `coroutine`, or `debug`.
* **Banned Runtimes:** Global functions capable of dynamic script evaluation (`load`, `loadstring`, `dofile`, `loadfile`) are disabled.
* **Require Isolation:** `require("lib.util")` loads `lib/util.lua` (or `lib/util/init.lua`) from the plugin's own folder, runs it once and caches the result in a table private to the plugin. It works for untrusted and trusted plugins alike (it does not need the `package` library). Module names are plain dotted identifiers (letters, digits, `_`, `-`); `..`, separators, symlinks and junctions cannot reach files outside the plugin folder.
* **No Commands:** Any call to `pairee.fs.spawn()` will immediately throw a runtime error.
* **Filesystem Jail:** `pairee.fs` can only read inside the plugin's own directory and its private data directory (`pairee.fs.data_dir()`), and can only write inside the data directory. This applies whether or not Secure Mode is enabled; `..` segments and symlinks cannot escape it.
* **Preview exception:** `pairee.image.show`, `pairee.image.info` and `pairee.preview_code` also accept the file being previewed; they hand it to the host and return an opaque widget (or the image size), never its content. `pairee.image.precache` decodes pixels the plugin can read back, so it only takes sources inside the jail.
* **Clipboard:** an untrusted plugin may write the clipboard but not read it (reading needs a trusted plugin outside Secure Mode).

```toml
# In pairee.toml
[plugins.csv-preview]
name    = "csv-preview"
trusted = false       # No system access needed (Untrusted sandbox mode)

[plugins.git-status]
name    = "git-status"
trusted = true        # Requests permission to spawn external tools (Trusted mode)
```

### 13.2 Trusted Mode (`trusted = true`)
When a plugin is explicitly trusted by the user, it runs in **Trusted Mode**:
* The plugin can use Lua's standard file and module management (`io`, `os`, `package`, `load`) — outside Secure Mode only (13.3).
* The plugin can execute external system commands using the `pairee.fs.spawn()` API.

### 13.3 Global Secure Mode (`secure_mode = true`)
To prevent data exfiltration, users can enable a global **Secure Mode** in their settings. This mode acts as an overriding runtime firewall:
* **Activation:** Configured in `pairee.toml` under `[settings]` -> `secure_mode = true`. This configuration is read-only at runtime and completely immutable from Lua.
* **Restricted standard library:** a trusted plugin gets the same Lua libraries as an untrusted one plus `coroutine` and a clock-only `os` (`os.time`, `os.date`, `os.clock`, `os.difftime`): no `io`, `package`, `load` / `loadfile` / `dofile`, `os.execute`, `os.remove`, `os.rename`, `os.getenv` or `os.exit`, which would bypass the jail and the process allowlist below.
* **Network & Socket Ban:** TCP/UDP socket creation and HTTP requests are completely disabled at the engine level for all plugins, including those with `trusted = true`.
* **Process Allowlist:** In Secure Mode a trusted plugin may only spawn (`pairee.Command`, `pairee.fs.spawn`) programs it declares in its manifest:
  ```toml
  [permissions]
  commands = ["git", "rg"]
  ```
  * The program must be given by **bare name** (no absolute/relative path); it is resolved through the absolute entries of `PATH` and that absolute path is executed. On Windows only `.exe` / `.com` are resolved (never `.bat` / `.cmd`).
  * Shells, interpreters, network utilities and command wrappers (`sh`, `bash`, `cmd`, `powershell`, `pwsh`, `python*`, `node`, `perl`, `ruby`, `lua`, `curl`, `wget`, `ssh`, `env`, `xargs`, `sudo`, ...) are **always** refused, even if declared, and also when a declared name is a symlink to one of them.
  * Undeclared commands fail with `Security violation: ... is not declared in the plugin manifest`. The Plugin Manager shows the declared commands in the plugin details so the user can review them before trusting the plugin.
* **Environment and working directory:** `Command:env` is limited to locale/terminal/colour variables and `Command:cwd` to folders inside the jail (see `pairee.Command`), so a declared tool cannot be turned into a code runner through `GIT_SSH_COMMAND`, `LD_PRELOAD`, `PAGER` and similar variables.
* **Arguments:** options that make a declared tool run another program are refused (Git config keys naming programs via `-c` or `git config`, `--upload-pack`, `--exec`, `--git-dir`, `rebase -x`, `difftool -x`, `grep -O`, `submodule foreach`, `bisect run`, `filter-branch`, `send-email`; `tar --to-command` / `-I`; `rg --pre`; `fd --exec`; `fzf --preview`; ...). `git` always runs with `-c safe.bareRepository=explicit` before the plugin's arguments.
* **FS Sandboxing:** For trusted plugins, file **reads** (`pairee.fs`) are locked to the active workspace folder, the user's config and cache directories, and the plugin's own and data directories. **Writes** are limited to the workspace, the cache and the plugin's data directory: Pairee's config directory (`config.toml`, `keybindings.toml`, associations, the user menu, `plugins/`, `plugins.lock`) is read-only, so a plugin cannot switch Secure Mode off or trust itself. Nothing inside a `.git` folder is writable (its `config` and hooks run programs). Untrusted plugins keep their stricter jail (13.1).
* **Fails closed:** if `config.toml` cannot be parsed, plugins run in Secure Mode until it is fixed (the startup error says so).

### 13.4 Resource limits
Every plugin has a memory cap (128 MiB untrusted, 512 MiB trusted) and a CPU watchdog: each call Pairee makes into the plugin (`main.lua`, `setup`, `entry`, `peek`, hook callbacks) may run for at most 10 seconds, time spent inside C functions such as `string.rep` included. The clock only runs while the plugin is executing: while a call waits for a dialog, `pairee.sync`, a sleep or a child process it is paused. A call over budget is aborted with `plugin aborted: exceeded its execution time limit`, and `pcall` cannot catch it; the next call starts with a fresh budget.

### 13.5 Managing plugins
In the Plugin Manager (Installed tab) `c` or Enter opens the plugin's settings (section 18), `t` toggles a plugin's trust, `e` enables or disables it and `s` toggles the global Secure Mode. Changes are saved and applied at once: all plugins are reloaded with the new settings. Disabled plugins are not loaded at all, so none of their code or hooks run.

---

## 14. Testing Your Plugin Locally

1. **Place your plugin** folder under `~/.config/pairee/plugins/<name>/`.
2. **Register it** in `pairee.toml`.
3. Run Pairee and check the logs in `~/.cache/pairee/app.log` for execution events.

---

## 15. Developer Mode Tools & PR Submission TUI

To simplify packaging and validation of plugins, developers can enable a dedicated developer suite in `pairee.toml`:

```toml
# In pairee.toml
[developer]
developer_mode = true   # Gates access to CLI developer commands and PR screens
```

### 15.1 CLI Developer Commands & Auto-Detection
With developer mode enabled, the following commands are available:
* `pairee developer format <path>`: Formats all Lua files in the plugin directory according to Pairee's standard.
* `pairee developer validate <path>`: Lints scripts, checks `manifest.toml` schemas, validates Lua syntax, and enforces strict cross-platform naming safety, name consistency, localized files coverage, translation key sync, and text encoding checks.
* `pairee developer package <path>`: Scans the directory structure, executes the validation suite, and packages the plugin:
  * **Auto-Detects Languages:** Scans the `lang/` directory for TOML files and registers them in the manifest.
  * **Auto-Detects Plugin Category:** Inspects references in `main.lua` to identify the plugin type.
  * **Generates Hashes:** Automatically generates/updates the version-specific `sha256.sum` file containing SHA-256 hashes of every file in the directory. (Note: this does not create a compressed archive; Pairee registry plugins are distributed as raw folders).
  * **License Detection & Generation:**
    - Checks for an existing `LICENSE` file (case-insensitive) in the plugin directory.
    - If a license file exists but the manifest `license` field is empty, the tool prompts you to specify the license name (or defaults to `"Custom"` if non-interactive).
    - If no license file is found, the tool automatically assigns the `"MIT"` license to your manifest and writes a standard MIT `LICENSE` file containing your copyright details in the plugin workspace.

### 15.2 TUI Developer Tools & PR Submission Wizard
The interactive TUI developer tools (accessible via **Top Menu Bar (F9) → Files → Plugin commands** → `Developer Tools` tab when `developer_mode = true` is enabled in `pairee.toml`) provide the following suite of utilities:

* **Active Plugin Selection (Option 0):** Opens an interactive modal selection list displaying all detected development plugins. It lists all plugins found under `plugins_dev_dir` and also scans the active directories of Panel 1 and Panel 2 for a `manifest.toml` file, letting you choose the targeted plugin from the list with your keyboard.
* **Initialize boilerplate (Option 1):** Guides the developer step-by-step to create a new plugin skeleton under your development directory. Upon successful creation, the new plugin is automatically set as the active plugin (Option 0) and Option 1 is disabled.
* **Audit (Lint) (Option 2):** Scans the active plugin folder and performs syntax and security audits on Lua and manifest files.
* **Package (Option 3):** Prepares a local temporary clone of the `plugin-registry` branch, copies all plugin assets, builds/updates the registry index entries, and displays the exact absolute path where it was packaged in the registry cache.
* **Install Local (Option 4):** Copies the active plugin to the local plugins runtime folder and registers it in `plugins.lock` for immediate TUI testing.
* **Submit Plugin (GitHub PR) (Option 5):** Submission wizard that collects descriptions, git commits, and optional GitHub PATs to automatically push to a fork and open a Pull Request.

### 15.3 Developer Validation Troubleshooting Guide
When running `pairee developer validate <path>` (or the CI validator), you may encounter the following validation errors. Here is how to fix them:

| Error / Warning Code | Root Cause | Solution |
|----------------------|------------|----------|
| `ERR_INVALID_NAME_CHAR` | File/folder name contains spaces, capitals, or special characters. | Rename files/directories to use lowercase alphanumeric, dots, dashes, and underscores only. |
| `ERR_MANIFEST_NAME_MISMATCH` | The plugin root directory name doesn't match `name` in `manifest.toml`. | Rename the root folder to match the manifest name exactly. |
| `ERR_LANG_FILE_MISSING` | A locale declared in `languages` is missing its `lang/<locale>.toml` file. | Create the missing `.toml` file under the `lang/` directory, or remove the locale from the manifest. |
| `ERR_HELP_FILE_MISSING` | A locale declared in `languages` is missing its `help/<locale>.md` file. | Create the missing `.md` file under the `help/` directory. |
| `ERR_DEFAULT_LANG_MISSING` | The specified `default_language` is missing its translation/help files. | Ensure both `lang/<default_lang>.toml` and `help/<default_lang>.md` exist. |
| `WARN_KEY_MISALIGNMENT` | Translation key is present in some language files but missing in others. | Update all `lang/*.toml` files to define the same keys to prevent runtime UI bracket placeholders. |
| `ERR_INVALID_ENCODING` | A file is not encoded in UTF-8. | Re-save the file using UTF-8 encoding in your text editor. |

### 15.4 Plugin Template Branch (`plugin-template`)

Pairee maintains a dedicated orphan git branch called **`plugin-template`** in its own repository. This branch holds the canonical, always-up-to-date boilerplate files for new plugins and is completely invisible to end-users — it never appears in any plugin list inside the TUI.

#### How It Works

When a developer uses the **Initialize New Plugin** wizard (either via the TUI Developer Tools tab or `pairee developer init`), the tool:

1. **Locates the Pairee repository** on disk by walking up from the running binary's location until a `.git` directory is found. Alternatively, the `PAIREE_REPO_DIR` environment variable can be set to point to the repository explicitly.
2. **Extracts files** directly from the `plugin-template` branch using `libgit2` (no external `git` binary needed). Every file in the branch is written to the new plugin's target directory.
3. **Substitutes placeholders** in `manifest.toml` and `help/en.md`:
   - `PLUGIN_NAME` → the name entered by the developer
   - `PLUGIN_DESCRIPTION` → the description entered by the developer
   - `PLUGIN_AUTHOR` → the author entered by the developer
4. If the branch or repository is **not available** (e.g., installed as a standalone binary without the source repo), it gracefully falls back to generating files from the built-in localization strings — guaranteeing that plugin initialization always works.

#### Template Branch File Structure

```
plugin-template/
├── manifest.toml          # [plugin] block with PLUGIN_NAME, PLUGIN_DESCRIPTION, PLUGIN_AUTHOR tokens
├── main.lua               # Full Lua boilerplate with setup(), entry(), and peek()
├── icon.png               # 256×256 grey placeholder PNG
├── lang/
│   └── en.toml            # Default English translation keys
├── help/
│   └── en.md              # Help file with PLUGIN_NAME token
└── screenshots/
    └── screenshot1.png    # 640×480 grey placeholder PNG
```

#### PAIREE_REPO_DIR Environment Variable

Set this variable to tell Pairee where to find its source repository when the auto-detection fails:

```sh
export PAIREE_REPO_DIR=/path/to/pairee/repo
pairee
```

#### Editing the Template

To update the boilerplate files that all newly created plugins start from, check out the branch and edit them directly:

```sh
git checkout plugin-template
# edit files...
git add -A && git commit -m "feat: update plugin template"
git checkout master
```

> **Note:** Changes to the `plugin-template` branch do **not** affect `master` and vice versa — the branch has no shared history with the main codebase.

---

## 16. Registry Submission Flow

1. **Enable Developer Mode** in `pairee.toml`.
2. Open the **Metadata Wizard TUI** or run `pairee developer package ~/.config/pairee/plugins/my-plugin` to validate your files, copy them to a local checkout of the `plugin-registry` branch, and update the central `registry/index.toml` catalog file.
3. Ensure the plugin meets the following mandatory publication requirements:
   - **Icon:** A PNG icon named `icon.png` must be located in the plugin's root directory (recommended sizes: `512x512` or `256x256` pixels).
   - **Screenshots:** A `screenshots/` directory must exist in the plugin's root containing at least one screenshot (PNG, JPG, or JPEG).
4. Run the packaging option to clone and prepare the `plugin-registry` branch locally, verify all signatures, copy code files, and build the checksums/index entries.
5. Execute the "Submit" option. The wizard will prompt you for:
   - **Commit Description:** A detailed description of the changes or features.
   - **GitHub Token (Optional):** If supplied, Pairee will automatically fork the repository, push the branch, and create a Pull Request. If skipped, Pairee will perform a local git commit and print the manual commands for you to copy and run to push your branch and open the PR yourself.
6. **Signatures are added by the registry, not by you.** Do not include `.minisig` files in your pull request. Once it is merged, the registry's signing workflow signs your `manifest.toml` and a new `index.toml` (with a serial and a timestamp that Pairee checks to refuse old or frozen copies of the index) with the Pairee key. Users can install your plugin once that signed index is published.

---

## 17. Translating Your Plugin (Localization)

Pairee plugins support isolated, self-contained translations. There is no need to edit the core application localization files.

### 17.1 Creating Language Files
Create a `lang/` subdirectory in your plugin's root folder, and place translation TOML files inside named after their ISO-639-1 language code:
```text
~/.config/pairee/plugins/my-plugin/
├── manifest.toml
├── main.lua
└── lang/
    ├── en.toml        # Default language translations
    └── es.toml        # Spanish translations
```

Inside the translation files (e.g. `lang/es.toml`), write your keys and messages:
```toml
[messages]
hello = "Hola {name}!"
error_vcs = "Error al ejecutar comando Git"
```

In your `manifest.toml`, specify your default language:
```toml
default_language = "en"
```

### 17.2 Using Translations in Lua
Use the global `pairee.t()` function to translate keys dynamically:
```lua
-- Simple translation lookup:
local error_msg = pairee.t("messages.error_vcs")

-- Translation with variable interpolation:
local greet = pairee.t("messages.hello", { name = "Iván" }) -- "Hola Iván!"
```

### 17.3 Fallback Mechanics
When `pairee.t()` resolves a key:
1. It checks the active locale of the user's Pairee application (e.g., Spanish `es`).
2. If `lang/es.toml` has the key, it uses it.
3. If not found or if `lang/es.toml` is missing, it falls back to the plugin's `default_language` (e.g., `lang/en.toml`).
4. If still missing, it returns the raw key identifier wrapped in brackets: `"[messages.hello]"` (allowing you to easily debug missing translations).

---

## 18. Custom Settings Configuration

Plugins can declare their own settings. Users edit them in the Plugin Manager without editing Lua code: `c` or Enter on an installed plugin opens a form with a checkbox per `bool` setting and a text field for the others.

### 18.1 Defining settings_schema
In your plugin's `manifest.toml`, define a `[settings_schema]` table containing the configurable options. Each option must specify a `type` (`bool`, `string`, `integer` or `number`), a `default` value, and a brief `description`. The user's value is converted with the declared type (without `type`, the type of `default` is used); a value that does not fit the type is ignored and the default is used:
```toml
[settings_schema]
show_hidden = { type = "bool", default = false, description = "Show hidden VCS files" }
git_path    = { type = "string", default = "git", description = "Custom Git executable path" }
max_depth   = { type = "integer", default = 3, description = "Max directory recursion depth" }
```

The form shows the focused setting's `description` and default. Save checks every value against its type (an invalid one is named and focused); Defaults puts every field back to the schema default. Only values that differ from the defaults are stored, in `config.toml` under `[settings.plugin_settings.<plugin>]`, and the plugins are reloaded so the new values apply at once.

### 18.2 Accessing Settings in Lua
The resolved settings values are made available directly to your Lua script inside the read-only global table `pairee.settings`:
```lua
-- Read settings in main.lua:
local is_hidden_visible = pairee.settings.show_hidden
local depth = pairee.settings.max_depth or 3

if is_hidden_visible then
  -- Perform action
end
```

---

## 19. Plugin Help Documentation (F1 Help)

To document keybindings, commands, and functionality for your users, you can integrate your documentation directly into Pairee's built-in `F1` help viewer.

### 19.1 Creating Help Markdown Files
Add a `help/` subdirectory to the root folder of your plugin, and place help markdown files named after their ISO-639-1 language code inside:
```text
~/.config/pairee/plugins/my-plugin/
├── manifest.toml
├── main.lua
└── help/
    ├── en.md          # English documentation (Default fallback)
    └── es.md          # Spanish documentation
```

Write the help contents using standard markdown:
```markdown
# My Git Status Plugin

This plugin displays the current Git branch and modification state in your panel header.

## Keybindings
* `Ctrl+G`: Refresh Git status
* `Ctrl+Shift+G`: Commit current panel changes

## Settings
Enable `show_hidden` in your Pairee settings to monitor ignored Git files.
```

### 19.2 UI Display
When users open the help menu by pressing `F1`, the system displays a sidebar or tab called **"Plugins Help"** and lists all active plugins containing valid help files inside their `help/` directories. Selecting your plugin displays your formatted markdown documentation (resolved based on the user's active locale or falling back to the default language) directly in the main reader pane.

---

## 20. Manifest Reference

```toml
name          = "my-plugin"
version       = "1.0.0"
description   = "My plugin description"
author        = "your-github-handle"
license       = "MIT"
type          = "hook"               # Auto-detected by developer tool
min_pairee    = "0.7.0"
requires_trust = false

# Language integration support
default_language = "en"
languages     = ["en", "es"]         # Auto-detected from lang/ directory

# Custom settings schema
[settings_schema]
show_hidden = { type = "bool", default = false, description = "Show hidden VCS files" }
git_path    = { type = "string", default = "git", description = "Custom Git executable path" }

[[commands]]
id = "run"
title = "Run"
[commands.keys]
default = "Ctrl+Alt+r"

# Programs the plugin may spawn when Secure Mode is on (bare names, see 13.3)
[permissions]
commands = ["git"]
```

---

## 21. Best Practices & Conventions

| Convention | Guidance |
|------------|----------|
| **Plugin naming** | Use `<name>.pairee` style for registry plugins (e.g. `git.pairee`, `fzf.pairee`) |
| **Namespaced events** | Prefix pub/sub events with your plugin name: `my-plugin:event` |
| **Decline gracefully** | In `peek()`, check the file type and return early if not applicable |
| **Async-first** | Avoid blocking calls. Use `pairee.Command` and async patterns; put slow preview work in `preload` |
| **Error notifications** | Show `pairee.app.notify` with level `"error"` on failures — never silently fail |
| **Trust transparency** | If your plugin needs `trusted = true`, document it clearly in the manifest |
| **SemVer discipline** | Bump MAJOR only on breaking behavioral changes |
| **Minimal surface** | Expose only what the plugin needs — less surface = less attack surface |

---

*See also: [docs/technical/plugin-system-design.md](technical/plugin-system-design.md) for the Rust engine architecture.*
*See also: [docs/technical/plugin-registry-spec.md](technical/plugin-registry-spec.md) for the registry schema and submission workflow.*
