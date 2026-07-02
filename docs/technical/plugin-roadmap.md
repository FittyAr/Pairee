# Pairee Plugin System — Evolution Roadmap

> **Internal Pairee design document. Enumerates the gaps in the current plugin system, specifies the new runtime surface (typed userdata, sync/async contexts, command builder, UI widgets, dialogs, utils), and lays out a phased implementation plan (M0–M5).**

---

## Table of Contents

1. [Executive Summary](#1-executive-summary)
2. [Current State Inventory](#2-current-state-inventory)
3. [Gap Inventory](#3-gap-inventory)
4. [Architectural Foundations](#4-architectural-foundations)
5. [New API Surface by Area](#5-new-api-surface-by-area)
6. [Secure-Mode Mapping for New APIs](#6-secure-mode-mapping-for-new-apis)
7. [Migration and Backward Compatibility](#7-migration-and-backward-compatibility)
8. [Acceptance Test Plugins](#8-acceptance-test-plugins)
9. [Implementation Roadmap](#9-implementation-roadmap)
10. [Appendix A — Pairee Current → Proposed API Mapping](#appendix-a--pairee-current--proposed-api-mapping)
11. [Appendix B — Pairee Files Affected](#appendix-b--pairee-files-affected)
12. [Appendix C — Reference Material](#appendix-c--reference-material)

---

## 1. Executive Summary

Pairee ships a working, sandboxed Lua plugin system built on `mlua`. The current surface (`pairee.app`, `pairee.fs`, `pairee.ui`, `pairee.ps`, `pairee.log`, `pairee.sync`) covers the basic use cases — previewers, hooks, commands, pub/sub, settings, i18n, F1 help, a developer TUI wizard, and a registry with SHA-256 distribution.

The current surface is, however, **incomplete at the runtime level**: the values passed to plugins are plain Lua tables with no methods, no metamethods, and no rich metadata; every plugin runs in a worker thread with a one-shot state snapshot; the dialog primitives are stubbed and never show a real dialog; the `Command` builder does not exist (only a fire-and-forget `spawn`); the UI widget set is six plain-table constructors; live state (`cx`), theme access (`th`), preferences (`rt`), and keymap access (`km`) are entirely missing; and the `preload`/`seek` methods documented in the developer guide are not actually routed.

This document enumerates **14 specific gaps** in the current implementation (each cited with `file:line` evidence), specifies the proposed new runtime surface in **6 areas** (typed userdata, async fs + Command, rich UI widgets, live context, real dialogs + action dispatch + utils, sync/async model with annotations), and lays out a **6-phase implementation plan (M0–M5)** of 1–3 weeks each — totalling ~10–14 weeks of work for one Rust+Lua developer.

The reference material at `example/reference/` (a vendored third-party plugin system) was used to validate the proposed design and to source concrete patterns for the more advanced features (typed userdata, sync/async bridge, widget builder, command streaming). No third-party name is exposed in Pairee's public API or in this document.

---

## 2. Current State Inventory

This section lists everything the current Pairee plugin system actually exposes and routes, based on a full read of `src/plugin/`. Every entry cites the source file and line.

### 2.1 Engine and lifecycle

| Concern | Status | Where |
|---|---|---|
| Plugin directory discovery | Implemented | `src/plugin/manager.rs:107-160` scans `~/.config/pairee/plugins/` for `*.pairee` folders |
| Manifest loading (`manifest.toml`) | Implemented | `src/plugin/loader.rs:24-35` (supports flat or nested `[plugin]` table); `69-119` reads + parses + version-checks |
| Sandbox creation | Implemented | `src/plugin/sandbox.rs:46-77` (`create_sandboxed_lua`) — `StdLib::TABLE \| STRING \| UTF8 \| MATH`, removes `load/loadstring/dofile/loadfile`, custom path-bounded `require` |
| Trusted mode flag | Implemented | `src/plugin/loader.rs:85`, `:150` (no `tokio::process::Command` execution when `trusted=false`) |
| Secure Mode | Implemented | `src/plugin/sandbox.rs:5-44` (27-command blacklist); `src/plugin/runtime/bindings/fs.rs:13-33` (FS restricted to workspace + config + cache) |
| Min-version check | Implemented | `src/plugin/loader.rs:78-82, 122-152` (custom semver parse, no `@since` granularity) |
| Plugin context channels | Implemented | `src/plugin/manager.rs:87-104` (mpsc send/receive, 100-entry buffer) |
| Per-plugin Lua VM | Implemented | `src/plugin/sandbox.rs:46-77` (one `mlua::Lua` per plugin) |
| Plugin teardown | Partial | `src/plugin/registry.rs:78-99` (per-plugin task channel), no global unload API |

### 2.2 Currently routed plugin methods

`src/plugin/registry.rs:17-29` defines only three `PluginTaskRequest` variants:

| Variant | Purpose | Notes |
|---|---|---|
| `Peek { job, reply_tx }` | Preview rendering | Job carries `file_path, area_width, area_height, skip` (no `File` userdata, no `mime`) |
| `ExecuteCommand { args }` | Command plugin invocation | Routed via `run_command` |
| `EmitEvent { name, data }` | Pub/sub delivery | JSON string payload |

`src/plugin/registry.rs:104-149` (`execute_peek_internal`) builds a Lua table with only `file.url` and `file.path` for the `peek` job — **no `cha`, no `mime`, no `is_hidden`, no `is_exec`, no timestamps, no `perm()`**.

### 2.3 Currently exposed Lua globals

`src/plugin/runtime/standard.rs` and `src/plugin/runtime/bindings/*.rs` register:

| Lua path | Status | Notes |
|---|---|---|
| `pairee.app.cwd` | Read (snapshot) | `src/plugin/runtime/bindings/app.rs:9-29` reads from `_current_snapshot` |
| `pairee.app.cd` | Write | `app.rs:31-42` (sends `PluginRequest::Cd`) |
| `pairee.app.focus` | Read (snapshot) | `app.rs:45-60` |
| `pairee.app.set_focus` | Write | `app.rs:63-73` (sends `PluginRequest::SetFocus`) |
| `pairee.app.notify` | Write (real) | `app.rs:75-86` → `manager.rs:191-194` opens a real `PopupType::Info` |
| `pairee.app.confirm` | **STUB** | `app.rs:88-111` → `manager.rs:207-215` always returns `true` |
| `pairee.app.input` | **STUB** | `app.rs:113-136` → `manager.rs:215-223` always returns `default` |
| `pairee.app.hovered` | Read (snapshot) | `app.rs:139-153` |
| `pairee.fs.read` | Sync, blocking | `bindings/fs.rs:43-50` uses `std::fs::read_to_string` |
| `pairee.fs.write` | Sync, blocking | `bindings/fs.rs:52-60` uses `std::fs::write` |
| `pairee.fs.exists` | Sync, blocking | `bindings/fs.rs:62-69` |
| `pairee.fs.stat` | Sync, blocking | `bindings/fs.rs:71-96` |
| `pairee.fs.list` | Sync, blocking | `bindings/fs.rs:98-127` (no glob/limit/resolve options) |
| `pairee.fs.spawn` | Fire-and-forget | `bindings/fs.rs:130-161` (no streaming, no stdin, no env, no kill) |
| `pairee.fs.spawn_copy_task` | Async (UI-bound) | `bindings/fs.rs:163-176` (no plain `copy`/`mkdir`/`remove`/`rename`) |
| `pairee.ui.Paragraph` | Table-only | `bindings/ui.rs:4-12` returns plain table |
| `pairee.ui.Gauge` | Table-only | `bindings/ui.rs:14-23` |
| `pairee.ui.List` | Table-only | `bindings/ui.rs:25-33` |
| `pairee.ui.Table` | Table-only | `bindings/ui.rs:35-46` |
| `pairee.ui.Span` | Table-only | `bindings/ui.rs:48-57` |
| `pairee.ui.Line` | Table-only | `bindings/ui.rs:59-67` |
| `pairee.ps.sub/pub/unsub` | Implemented | `bindings/ps.rs:5-60` (per-VM `_callbacks` table, local only) |
| `pairee.log.{info,warn,error,debug}` | Implemented | `bindings/log.rs:1-37` |
| `pairee.sync(fn)` | Bridge | `bindings/sync.rs:5-57` (snapshot oneshot only) |
| `pairee.settings.*` | Read-only from manifest | `standard.rs:42-107` (`settings_schema` parsed from `manifest.toml`) |
| `pairee.t(key, vars)` | i18n | `standard.rs:109-191` (lang TOML, fallback to brackets) |

### 2.4 Current strengths to preserve

- **Author tooling** (no external system matches the same level):
  - Manifest-based `settings_schema` rendered in the config TUI
  - Per-plugin i18n via `lang/<locale>.toml`
  - F1 help integration via `help/<locale>.md`
  - `plugin-template` orphan branch with placeholder substitution
  - `pairee developer` CLI (`format`, `validate`, `package`) with strict cross-platform naming and encoding rules
  - TUI Developer Tools wizard (init, audit, package, install, submit)
  - Registry with SHA-256 file-by-file verification and a CLI suite (`search`, `list`, `install`, `update`, `check-updates`, `pin`, `remove`)
- **Sandboxing**: stricter than typical — explicit removal of `load/loadstring/dofile/loadfile` plus custom `require` (no `io`/`os`/`package`/`debug`/coroutine in untrusted)
- **Secure Mode**: 27-command blacklist + FS boundary — beyond what most comparable systems offer

---

## 3. Gap Inventory

Each gap is cited with `file:line` evidence. The numbers (G1–G14) are used throughout the rest of the document.

### 3.1 No typed userdata — G1

`src/plugin/manager.rs:18-26` defines `FileEntrySnapshot { name, url, path, size, is_dir, is_symlink }` as a plain serializable struct. When sent to Lua, it becomes a plain table (`src/plugin/registry.rs:117-124` builds a table with only `url` and `path` — no `cha`, no `mime`, no `is_hidden`, no `is_exec`, no timestamps, no `perm()`).

Plugins receive a thin table and must derive everything else themselves. There is no way to add methods, no metamethods (`__eq`, `__tostring`, `__concat`), and no way to extend the type from outside the Rust core.

### 3.2 No sync context / no live state — G2

`src/plugin/runtime/bindings/sync.rs:8-56` is the only bridge. There is no global equivalent of a live context (call it `cx`). Plugins read `pairee.app.cwd()` (which reads from `_current_snapshot` set by `sync.rs:41`) but cannot iterate the panel, list selected files, get cursor position, read tasks progress, or query the other panel.

This is the deepest single gap: every advanced UI feature (previewers that depend on the cursor, hooks that depend on selection, pluggable panels) is blocked.

### 3.3 No `Command` builder — G3

`src/plugin/runtime/bindings/fs.rs:130-161` is a single `spawn(cmd, args)` that calls `tokio::process::Command::new(&cmd).args(&args).output().await` and returns `{stdout, stderr, status}`. No stdin, no env, no cwd, no streaming, no kill, no INHERIT/PIPED/NULL distinction. No `Child` object. No `Output`/`Status` userdata.

This alone prevents the entire class of interactive plugins (fuzzy finders, history navigators, REPL integrations) that depend on bidirectional stdin/stdout control.

### 3.4 `fs.*` is sync-blocking and minimal — G4

`src/plugin/runtime/bindings/fs.rs:43-127` uses `std::fs::read_to_string`, `std::fs::write`, `std::fs::metadata`, `std::fs::read_dir` — **synchronous I/O inside `create_function`/`create_async_function`**. These block the plugin worker thread (and indirectly the tokio runtime worker if scheduled on one). Missing: `mkdir`, `remove`, `rename`, `copy` (only `spawn_copy_task` exists, which is UI-bound), `read_dir({glob, limit, resolve})`, `Access`/`Fd` builder, typed errors.

### 3.5 UI widgets are table-stubs — G5

`src/plugin/runtime/bindings/ui.rs:1-69` exposes 6 constructors returning plain tables with a `type=...` discriminator: `Paragraph`, `Gauge`, `List`, `Table`, `Span`, `Line`. The widget values are passed back to the main thread via `PluginRequest::UpdatePluginWidget { path, widget }` and deserialized by serde into `PluginWidget` (`src/app/state/types/...`). No styling, no layout, no Rect positioning, no borders, no alignment, no wrap, no image rendering.

### 3.6 Pub/sub is local-only and shallow — G6

`src/plugin/runtime/bindings/ps.rs:5-60` uses a per-VM Lua table `_callbacks` (line 8) to store subscriptions. Cross-plugin communication is achieved by Rust `emit_event` (`src/plugin/hooks.rs:4-17`) iterating all loaded plugins and calling each one's VM. There is no remote bridge, no serialization layer, no equivalent of a cross-instance delivery system, no `pub_to`. The `pairee.ps.unsub` (line 47-57) sets the event entry to `nil` but does not handle the case where multiple plugins subscribed to the same event.

### 3.7 `cd` and `set_focus` are the only action-dispatch entry points — G7

`src/plugin/runtime/bindings/app.rs:31-73` registers only `cd(path)` and `set_focus(side)`. The `PluginRequest` enum (`src/plugin/manager.rs:42-85`) has 11 variants, but they are all read operations (Notify, Cd, SetFocus, Confirm, Input) or special-purpose (SpawnCopyTask, UpdatePluginWidget, PluginMenuLoaded, DevPluginScan, GetStateSnapshot). There is no `EmitAction(name, args)` variant that would let a plugin trigger any of Pairee's `Action` enum (defined in `src/keybindings/actions.rs`).

### 3.8 Dialogs are stubbed — G8

`src/plugin/manager.rs:207-223`:
```rust
PluginRequest::Confirm { title, msg, reply_tx } => {
    log::info!("Plugin confirm dialog requested: {} - {}", title, msg);
    let _ = reply_tx.send(true);  // <-- always true; user never sees a dialog
}
PluginRequest::Input { title, default, reply_tx } => {
    log::info!("Plugin input dialog requested: {} - {}", title, default);
    let _ = reply_tx.send(default);  // <-- always default
}
```

Both dialogs are logged and a canned response is sent. The user never sees a real dialog. The `notify` path (`manager.rs:191-194`) at least sets `state.active_popup = Some(PopupType::Info(...))`, so notifications work.

### 3.9 Preloader and seek not routed — G9

`src/plugin/registry.rs:17-29` defines only `PluginTaskRequest::{ Peek, ExecuteCommand, EmitEvent }`. `Preload` and `Seek` are documented in `docs/plugin-dev-guide.md:8` but not implemented. The `PreviewJob` (`registry.rs:10-15`) only carries `file_path, area_width, area_height, skip` — no `file` userdata, no `mime`.

### 3.10 Event coverage is thin — G10

`docs/plugin-dev-guide.md:38` lists only `on_cd`, `on_hover`, `on_key`, `on_focus`. A richer set of internal events (`reveal`, `select`, `toggle`, `yank`, `paste`, `update_yanked`, `update_mimes`, `update_files`, `tasks:update_succeed`, `update_peeked`, `update_spotted`) is not exposed to plugins.

### 3.11 Per-plugin settings TUI rendering is partial — G11

Pairee has `settings_schema` (`docs/plugin-dev-guide.md:18`) and a TUI section that renders it, but the rendering code is in the broader config UI, not in the plugin module. The TUI is documented but the `src/ui/popup.rs` `PluginMenu` variant (`src/plugin/manager.rs:256-268`) only handles plugin install/update, not per-plugin settings form rendering.

### 3.12 No image preview — G12

`Cargo.toml:25` has the `image` crate (0.25.10), and the core F3 viewer renders images, but no Lua API exposes image preview. The terminal image adapter stack (Chafa, iTerm2, Sixel, Überzug) is also not exposed to plugins.

### 3.13 Missing utility functions — G13

None of the following exist today: `clipboard`, `sleep`, `time`, `hash`, `quote` (shell escape), `target_os/family`, `uid/gid/user_name/group_name/host_name`, `json_encode/decode`, `image_show/precache/info`.

### 3.14 No mutable per-plugin state — G14

`pairee.settings` is read-only, derived from the manifest. There is no equivalent of a per-plugin mutable table that persists between calls. A plugin that needs a counter, a cache, or a "last seen" timestamp must re-derive it on every call.

---

## 4. Architectural Foundations

The 14 gaps fall into 6 thematic areas. The areas build on three shared architectural foundations introduced first.

### 4.1 Foundation F1 — Typed userdata with metamethods and builder pattern

**Why**: every area A, B, C, D depends on it. Plugins must receive real Rust types (not tables) so they can call methods, compare for equality, concatenate, and convert to string.

**Design**:
- Every value passed to Lua that has more than 2 fields is a `mlua::UserData`.
- Every type has `__tostring` (so `tostring(x)` works), and most have `__eq` (so `a == b` works) and `__concat` (so `..` works).
- Every collection value is iterable via `__pairs` or `__index` (so `#files`, `for _, f in ipairs(files)` work).
- Every builder method returns the userdata itself (not a new value), enabling chaining.
- A small `add_cached_field` shim memoizes derived fields (e.g. `Url.name`) on the userdata handle — the first call computes, subsequent calls return the cached `mlua::Value` stored in the userdata's named user value.

### 4.2 Foundation F2 — Sync context machinery

**Why**: areas D (`cx`, `rt`, `th`, `km`) and F (sync/async bridge) depend on it. Previewers, hooks, and pluggable UI components need to read live state, not a one-shot snapshot.

**Design**:
- A `Runtime` struct stored as Lua app data: `{ blocking: bool, frames: VecDeque<RuntimeFrame>, blocks: HashMap<String, Vec<Function>> }`.
- A `runtime_scope!(lua, id, block)` macro pushes a frame, sets `blocking=true`, runs the block, pops the frame.
- Sync plugin execution runs in the same thread as the main event loop, inside a `runtime_scope!`. The main thread holds the only `Lua` state.
- Async plugin execution runs in isolated Lua states (no `cx`, no `rt`, no `th`); these plugins use `pairee.sync(fn)` to bridge into the sync context.
- The `blocking` flag is checked at the top of every interactive API (`pairee.which`, `pairee.input`, `pairee.confirm`) — if `true`, the call throws to prevent re-entrancy.
- `pairee.sync(fn)` and `pairee.async(fn)` are the bridges. In sync context, the bridge calls the function directly. In async context, the bridge serializes args, sends them to the main thread via a callback channel, awaits the result.

### 4.3 Foundation F3 — Standard `(value, Error?)` multi-return convention

**Why**: every async API in areas B and E depends on it. Without typed errors, plugins can't distinguish between "permission denied" and "file not found" and "network down".

**Design**:
- Every async function that can fail returns `(value, Error?)`. On success, `Error` is `nil`. On failure, `value` is `nil` and `Error` is a userdata with `code` (i32 | nil) and `kind` (string | nil) fields plus a `__tostring` that returns the OS error message.
- A helper `Err(s, ...)` Lua function in `src/plugin/runtime/presets/ya.lua` constructs an `Error.custom` from a format string.
- Backward compatibility: existing `pairee.fs.read`/`write` (which currently throw on error) keep their throwing behavior; only new APIs use the tuple.

### 4.4 Foundation F4 — `pairee.state` mutable per plugin

**Why**: G14 cannot be closed by extending `pairee.settings` (read-only by design). It is a distinct concept that sits alongside `pairee.settings` and `pairee.t()`.

**Design**:
- `pairee.state` is a Lua table per plugin instance, mutable, persists across calls, stored in `Runtime::blocks` keyed by plugin name.
- The state is first-class: it can be passed to `pairee.sync(function(state) ... end)` as the first argument.
- The state is destroyed when the plugin is unloaded.

---

## 5. New API Surface by Area

The 6 areas build on the 4 foundations above. Each area lists the new API, its Lua signature, its Rust module, and the implementation phase.

### 5.A Typed UserData (foundation)

**A1. `Url` userdata**
- Constructors: `Url("path" | "sftp://host//path")`, `Url(other_url)` (clone).
- Fields (cached): `path`, `name`, `stem`, `ext`, `urn`, `base`, `parent`, `scheme`, `domain`.
- Fields (direct): `is_regular`, `is_search`, `is_archive`, `is_absolute`, `has_root`.
- Methods: `join(other)`, `starts_with(base)`, `ends_with(child)`, `strip_prefix(base)`, `into_search(domain)`.
- Metamethods: `__eq`, `__tostring`, `__concat`.
- **Rust**: `src/plugin/types/url.rs`.

**A2. `Path` userdata**
- Constructor: `Path.os("string")` (or `Path("string")`).
- Fields: `name`, `stem`, `ext`, `parent`, `is_absolute`, `has_root`.
- Methods: `join`, `starts_with`, `ends_with`, `strip_prefix`.
- Metamethods: `__eq`, `__tostring`, `__concat`.
- **Rust**: `src/plugin/types/path.rs`.

**A3. `Cha` userdata (file characteristics)**
- Constructor: `Cha{...}` from table.
- Fields: `mode` (u16), `is_dir`, `is_hidden`, `is_link`, `is_orphan`, `is_dummy`, `is_block`, `is_char`, `is_fifo`, `is_sock`, `is_exec`, `is_sticky`, `len`, `atime`, `btime`, `mtime`, `uid`, `gid`, `nlink`.
- Methods: `perm()` → string (Unix permission representation; `nil` on Windows), `hash(long?)` → hex.
- **Rust**: `src/plugin/types/cha.rs`. Internal bitflags `ChaKind` (FOLLOW/HIDDEN/SYSTEM/DUMMY) + `ChaMode` (T_FILE/T_DIR/T_LINK/T_BLOCK/T_CHAR/T_FIFO/T_SOCK + S_SUID/S_SGID/S_STICKY + 9 perm bits).

**A4. `File` userdata** (the main entry point)
- Constructor: `File{url=Url, cha=Cha}` or `File(other_file)` (clone).
- Fields: `cha`, `url`, `link_to`, `name`, `path`, `cache`.
- Methods: `icon()`, `size()`, `mime()`, `prefix()`, `style()`, `is_selected()`, `is_yanked()`, `found()`, `hash()`.
- `File` derefs to `Cha`, so all of `Cha`'s fields and methods are also accessible directly on `File`.
- **Rust**: `src/plugin/types/file.rs` with `impl Deref<Target = Cha>`.

**A5. `Error` userdata**
- Constructors: `Error.custom("msg")`, `Error.fs({kind, code, message})`.
- Fields: `code` (i32 | nil), `kind` (string | nil).
- Metamethods: `__tostring`, `__concat`.
- All `fs.*` and `Command.*` methods return `(value, Error?)` multi-value tuples.
- **Rust**: `src/plugin/types/error.rs`. Add helper `Err(s, ...)` in `src/plugin/runtime/presets/ya.lua`.

**A6. SFTP URL support** (reuses the existing `ssh2` crate in `Cargo.toml:37,42`)
- `Url("sftp://user@host:port//path")` parses into the URL scheme enum.
- The new VFS abstraction layer dispatches to `ssh2` for metadata/read/write.
- This is a substantial new capability (remote FS browsing) and ships in M2.

**A7. Image preview in plugins**
- `pairee.image.show(url, rect)` → reuses the existing `image` crate for decoding, integrates with the terminal image adapter stack (Chafa / iTerm2 / Sixel) via a new `pairee-tty-adapter` internal module.
- `pairee.image.precache(src, dist)` → resize + write to cache dir.
- `pairee.image.info(url)` → `{w, h, format, color}`.
- **Rust**: `src/plugin/runtime/bindings/image.rs`.

**A8. Typed `Error` everywhere**
- The `(value, Error?)` multi-return becomes the convention for every async function in `fs.*`, `Command.*`, and any future async API.
- **Backward-compat**: existing `pairee.fs.read`, `pairee.fs.write` etc. that return strings (currently throwing on error) keep their throwing behavior, but new APIs use the tuple.

**Phase**: **M2 (UserData foundation)**.

### 5.B Filesystem + Command (async, streaming)

**B1. Make `fs.*` async with `tokio::fs`**
- Replace `std::fs::read_to_string` etc. with `tokio::fs::read_to_string` so the plugin worker thread is not blocked.
- All new `fs.*` return `(value, Error?)`.

**B2. Add missing `fs.*` operations**
- `fs.mkdir(type, url)` where `type ∈ {"dir", "dir_all"}`.
- `fs.remove(type, url)` where `type ∈ {"file", "dir", "dir_all", "dir_clean"}`.
- `fs.rename(from, to)`.
- `fs.copy(from, to)` → bytes copied (background, cancellable).
- `fs.read_dir(url, {glob?, limit?, resolve?})` → `File[]`.
- `fs.unique(type, url)` → unique `Url`.
- `fs.cha(url, follow?)` → `Cha`.
- `fs.file(url)` → `File`.
- `fs.expand_url(value)` → `Url`.
- `fs.partitions()` → `Partition[]` (for the ChDrive UI integration).
- `fs.calc_size(url)` → `SizeCalculator` (yields Cha as it walks the tree).

**B3. `Command` builder**
- `Command("ls"):arg(...):cwd(...):env(k,v):stdin(Stdio):stdout(Stdio):stderr(Stdio):memory(max):spawn()→Child, :output()→Output, :status()→Status`.
- `Stdio ∈ {Command.NULL, Command.PIPED, Command.INHERIT}`.
- **Rust**: `src/plugin/runtime/bindings/process/command.rs`.

**B4. `Child` userdata**
- Methods: `:id()`, `:read(len)`, `:read_line()`, `:read_line_with({timeout})`, `:write_all(src)`, `:flush()`, `:wait()`, `:wait_with_output()`, `:try_wait()`, `:start_kill()`, `:take_stdin()`, `:take_stdout()`, `:take_stderr()`.
- `read_line` uses `tokio::select!` to race stdout vs stderr.
- **Rust**: `src/plugin/runtime/bindings/process/child.rs`.

**B5. `Output` and `Status` userdata**
- `Output { status: Status, stdout: string, stderr: string }`.
- `Status { success: bool, code: number? }`.

**B6. `Access` builder and `Fd` userdata**
- `fs.access():read(true):write(true):open(url) → Fd`.
- `Fd:write_all(bytes)`, `Fd:flush()`, `Fd:read(len)`.

**Phase**: **M2 (UserData) + M3 (Async fs/Command)**.

### 5.C UI Widgets (full ratatui)

**C1–C6. Basic widgets as userdata with builder pattern**
- `ui.Span("text")`, `ui.Line(...)`, `ui.Text(...)`, `ui.List(...)`, `ui.Paragraph(text)`, `ui.Table(...)` — same names as today, but returned as userdata with `:style(s)`, `:area(rect)`, `:fg(color)`, `:bg(color)`, `:bold()`, `:italic()`, `:underline()`, `:align(...)`, `:wrap(...)`, `:width()`, `:visible()`.
- `ui.Text.parse(ansi_string)` → `ui.Text` with ANSI escape sequences decoded.
- The `__call` of `ui.Span` also accepts another `ui.Span` (clone). `ui.Line`'s `__call` accepts string, Span, or table of mixed.
- **Rust**: each widget is a `mlua::UserData` in `src/plugin/runtime/bindings/ui/elements/{span,line,text,list,paragraph,table}.rs`. Builder methods return `ud` for chaining.

**C7. `ui.Style` userdata**
- `:fg(color)`, `:bg(color)`, `:bold()`, `:dim()`, `:italic()`, `:underline()`, `:blink()`, `:reverse()`, `:hidden()`, `:crossed()`, `:reset()`, `:patch(other_style)`, `:raw()`.
- Color: accept string (`"red"`, `"#ff0000"`, `"rgb(255,0,0)"`), `ui.Color(userdata)`, or `nil` (reset).
- `Style` is inherited by Span/Line/Text (chained method calls).

**C8. `ui.Layout` + `ui.Constraint` + `ui.Rect`**
- `ui.Rect{x,y,w,h}`, fields: `x, y, w, h, left, right, top, bottom`, method `:pad(Pad)`.
- `ui.Layout():direction(Layout.HORIZONTAL|VERTICAL):margin(n):constraints({...}):split(rect) → Rect[]`.
- `ui.Constraint.{Min, Max, Length, Percentage, Ratio, Fill}` factories.

**C9. `ui.Pad`, `ui.Pos`, `ui.Align`, `ui.Wrap`, `ui.Edge`**
- `ui.Pad(top,right,bottom,left)` with factory methods `Pad.left(n)`, `Pad.right(n)`, `Pad.top(n)`, `Pad.bottom(n)`, `Pad.x(n)`, `Pad.y(n)`, `Pad.xy(x,y)`.
- `ui.Pos { "top-center", x, y, w, h }` for positioning dialogs.
- `ui.Align.LEFT|CENTER|RIGHT`.
- `ui.Wrap.NO|YES|TRIM`.
- `ui.Edge.NONE|TOP|RIGHT|BOTTOM|LEFT|ALL` (bitmask).

**C10. `ui.Border`, `ui.Bar`, `ui.Clear`, `ui.Gauge`, `ui.Fill`**
- `ui.Border(Edge):type(Border.PLAIN|ROUNDED|DOUBLE|THICK|QUADRANT_INSIDE|QUADRANT_OUTSIDE):style(s):title(Line, Edge?):merge(bool)`.
- `ui.Bar(Edge):symbol(str):style(s)`.
- `ui.Clear(Rect)`.
- `ui.Gauge():ratio(0..1)|percent(n):label(span):style(s):gauge_style(s)`.
- `ui.Fill(rect):style(s)`.

**C11. `ui.Table` and `ui.Row`, `ui.Cell`**
- `ui.Table({Row, Row, ...}):header(Row):footer(Row):widths({Constraint, ...}):spacing(n):style(s):row_style(s):col_style(s):cell_style(s):row(n?):col(n?)`.
- `ui.Row({Cell, ...}):style(s):height(n):margin_t(n):margin_b(n)`.
- `ui.Cell` is a transparent wrapper (string/Span/Line/Text).

**C12. Renderable dispatch**
- A `Renderable` enum in Rust wraps each widget variant. Lua callbacks returning a `Renderable` (e.g., from a future `c:redraw(area)` pattern) get dispatched to the ratatui backend.
- **Rust**: `src/plugin/runtime/bindings/ui/renderable.rs`.

**Phase**: **M4 (UI widgets)**.

### 5.D Context (`cx`), Runtime (`rt`), Theme (`th`), Keymap (`km`)

**D1. `cx` global — sync-only**
- Set during sync context by the main thread; `nil` in async contexts.
- Tree: `cx.active` (Tab), `cx.tabs`, `cx.tasks`, `cx.yanked`, `cx.input`, `cx.which`, `cx.layer`.
- `cx.active` is a `Tab` with: `id`, `name`, `mode` (is_select/is_unset/is_visual), `pref` (sort_by, sort_sensitive, sort_reverse, sort_dir_first, show_hidden, linemode), `current` (Folder), `parent` (Folder?), `selected` (iterable File[]), `preview` (skip, folder), `finder` (filter string).
- `Folder` has: `cwd` (Url), `files` (Entries), `window` (Entries), `offset`, `cursor`, `hovered` (File?).
- `Entries` is windowed: `#entries`, `entries[i]` (1-indexed, windowed offset).
- `File` here is the userdata from §5.A4 with extra fields: `idx`, `is_hovered`, `in_current`, `in_preview`.
- **Rust**: `src/plugin/lives/`. Only registered in sync Lua state.

**D2. `rt` global — always available**
- `rt.args.entries`, `rt.args.cwd_file`, `rt.args.chooser_file`.
- `rt.term.light`, `rt.term.cell_size()` → `w, h`.
- `rt.mgr.{sort_by, sort_sensitive, sort_reverse, sort_dir_first, show_hidden, scrolloff, mouse_events}` (some mutable via `ArcSwap` for live updates; some read-only).
- `rt.plugin.{fetchers, spotters, preloaders, previewers}` (rule tables).
- `rt.preview.{wrap, tab_size, max_width, max_height, cache_dir, image_delay, image_filter, image_quality}`.
- `rt.tasks.{file_workers, plugin_workers, fetch_workers, preload_workers, process_workers, image_alloc, image_bound, suppress_preload}`.
- `rt.open.rules`, `rt.opener`, `rt.tty:queue(...)`, `rt.tty:flush()`.
- **Rust**: `src/plugin/runtime/bindings/rt.rs`.

**D3. `th` global — read-only live**
- `th.app`, `th.mgr`, `th.tabs`, `th.mode`, `th.indicator`, `th.status`, `th.which`, `th.confirm`, `th.spot`, `th.notify`, `th.pick`, `th.input`, `th.cmp`, `th.tasks`, `th.help`.
- Each leaf is a `ui.Style` userdata.
- In async/isolate contexts, materializes a snapshot at start.
- **Rust**: `src/plugin/runtime/bindings/th.rs`.

**D4. `km` global — read-only live**
- `km[layer_name]` → keymap section table for that layer.
- **Rust**: `src/plugin/runtime/bindings/km.rs`.

**D5. `ps` global — pub/sub with optional remote bridge**
- Keep `pairee.ps.{sub, pub, unsub}` for local. Add `pairee.ps.{sub_remote, pub_to, unsub_remote}` only if/when a cross-instance bridge is implemented (out of scope for M0–M4).

**D6. Sync context machinery**
- Introduce a `Runtime { blocking: bool, frames: VecDeque<RuntimeFrame>, blocks: HashMap<String, Vec<Function>> }` in app data.
- `runtime_scope!(lua, id, block)` macro sets `blocking=true`, pushes frame, runs, pops.
- `pairee.sync(fn)` and `pairee.async(fn)` following the sync-context pattern.
- This is the biggest single change in the proposal; it is gated on the userdata refactor.

**D7. Per-plugin mutable `pairee.state`**
- Distinct from `pairee.settings` (read-only, from manifest) and `pairee.t()` (i18n).
- Lifetime: per plugin instance, persists between calls.
- Implemented as a per-plugin Lua table stored in the `Runtime::blocks` map.
- **Phase**: **M3 (Async fs/Command + sync context)**.

**Phase**: **M3**.

### 5.E Dialogs, `pairee.emit`, utils

**E1. Real `pairee.input` and `pairee.confirm`**
- Fix the stub at `src/plugin/manager.rs:207-223`.
- New `pairee.input({pos, title, value, obscure, realtime, debounce})` returning `(value, event)` or `Recv` for realtime.
- New `pairee.confirm({pos, title, body})` returning boolean.
- Add new `PluginRequest::InputDialog` and `ConfirmDialog` variants that open the existing TUI input/confirm popups (`src/ui/popup.rs`).

**E2. `pairee.which({cands, silent})`**
- New key-prompt API.
- Returns 1-based index of selected candidate, or `nil`.
- **Rust**: new `PluginRequest::WhichPrompt` variant routed to a new `which` popup.

**E3. `pairee.emit(action, args)`**
- The general action dispatch: any `Action` from `src/keybindings/actions.rs` is callable.
- `pairee.emit("cd", {"/some/path"})` → triggers the `Cd` action.
- `pairee.emit("select", {url, state=true})` → triggers selection.
- `pairee.emit("reveal", {url})` → reveal in opposite panel.
- **Rust**: new `PluginRequest::EmitAction { name: String, args: serde_json::Value }`; the main loop looks up the action in the keybinding resolver and runs it.
- **Phase**: **M0 (scaffolding)** — small change, high leverage.

**E4. `pairee.file_cache({file, skip})`**
- Returns a cache URL (hash-based) for previewers. Prevents recursive caching of the cache file.
- **Rust**: new `PluginRequest::FileCache`.

**E5. `pairee.preview_code({area, file, mime, skip})` and `pairee.preview_widget(opts, widget)`**
- `preview_code` integrates the existing `pulldown-cmark` and adds syntax highlighting (no highlighter today; add `syntect` or shell-out).
- `preview_widget` writes a widget directly to the preview pane (extends `UpdatePluginWidget` to accept any `Renderable`).

**E6. `pairee.notify({title, content, timeout, level})`**
- Aligns with the standard signature. Extend the existing `Notify` request to take a structured payload.

**E7. `pairee.clipboard(text?)`**
- Get/set system clipboard.
- **Sandbox**: block `get` in secure mode; allow `set` only within workspace.

**E8. `pairee.quote(str, unix?)`**
- Shell-escape a string. Use the existing OS-specific escaping in `src/terminal/`.

**E9. `pairee.{sleep, time, hash, target_os, target_family, json_encode, json_decode, percent_encode, percent_decode}`**
- `sleep` is async (uses `tokio::time::sleep`); the rest are sync.
- `hash` uses XxHash3-128 for stable, fast hashing.
- `json_encode`/`decode` use the existing `serde_json`.

**E10. `pairee.{uid, gid, user_name, group_name, host_name}` (Unix-only)**
- On Windows, return `nil` (not error) so plugins can write portable code.
- Use the `uzers` crate (or `getpwuid`/`getgrgid` via `libc`).

**Phase**: **M1 (scaffolding + emit + dialogs fixes) + M4 (utils)**.

### 5.F Sync/Async, `@sync`/`@since` annotations, preloader/seek

**F1. Sync vs async contexts**
- Introduce a `Runtime` struct.
- Add `runtime_scope!` macro to set `blocking=true` during sync plugin execution.
- Async plugins get isolated slim Lua states without the live globals and with `th` materialized.

**F2. `@sync` / `@since` annotation parsing**
- Parse `--- @sync entry` and `--- @sync peek` from `main.lua` comments at load time.
- Parse `--- @since 0.7.0` and check at load time against `CARGO_PKG_VERSION`.
- Store on the plugin info struct in the registry.
- **Rust**: `src/plugin/loader.rs` extends `load_plugin` to read annotation lines from `main_content` before evaluation.
- **Phase**: **M3**.

**F3. `preload()` and `seek()` routing**
- Add `PluginTaskRequest::Preload { job, reply_tx }` and `PluginTaskRequest::Seek { job, reply_tx }` to `src/plugin/registry.rs:17-29`.
- `Preload` returns `(complete: bool, err: Option<String>)` (standard pattern).
- `Seek` synchronously updates `skip` and triggers another `Peek`.
- **Phase**: **M3**.

**F4. `pairee.sync(fn)` and `pairee.async(fn)`**
- `pairee.sync(fn)` — creates a sync block, callable from async context to bridge into sync state.
- `pairee.async(fn)` — spawns a function on the current thread's async local set.
- Both follow the same pattern: sync path is direct, async path uses a callback channel.
- **Phase**: **M3**.

---

## 6. Secure-Mode Mapping for New APIs

When Secure Mode is active (`pairee.toml` `[settings] secure_mode = true`), each new API must be classified:

| New API | Risk in Secure Mode | Recommended action |
|---|---|---|
| `pairee.clipboard` get | **High** (data exfiltration vector) | Block (return nil + warn) |
| `pairee.clipboard` set | Medium (could leak via paste) | Allow only if value is in workspace |
| `Command` builder with `INHERIT` stdio | Medium (could bypass blacklist via shell child) | Keep `is_command_safe` check; also block `INHERIT` stdio in secure mode |
| `Command` with `PIPED` stdio | Low (controlled by the plugin) | Allow |
| `pairee.image.show/precache/info` | Low (reads local files only) | Allow |
| `fs.read_dir` with `resolve: true` | Medium (could follow symlinks outside sandbox) | Force `validate_path` on result |
| `fs.create`, `fs.remove`, `fs.rename` | High (could escape workspace) | Restrict to workspace + config + cache dirs (same as current `validate_path`) |
| `fs.unique` | Low | Allow |
| `pairee.uid/gid/user_name/group_name/host_name` | Low (info disclosure) | Allow |
| `pairee.target_os/family`, `pairee.time` | Low | Allow |
| `pairee.hash` | Low | Allow |
| `pairee.quote` | Low (string op) | Allow |
| `pairee.json_encode/decode`, `percent_encode/decode` | Low | Allow |
| `pairee.emit(action)` | Low (actions are validated by the resolver) | Allow (but block emit to dangerous actions like `delete` in secure mode) |
| `pairee.which`, `pairee.input`, `pairee.confirm` | Low (local UI) | Allow |
| `pairee.preview_code/widget` | Low | Allow |
| `pairee.file_cache` | Low | Allow |
| `cx`, `rt`, `th`, `km` | Low (read-only state) | Allow (read-only) |

---

## 7. Migration and Backward Compatibility

Pairee already has a documented public API (`pairee.app.*`, `pairee.fs.*`, `pairee.ui.*`, `pairee.ps.*`). Changing `FileEntrySnapshot` to a `File` userdata, renaming `pairee.fs.list` to `pairee.fs.read_dir`, etc. will break existing plugins.

**Recommended dual-phase migration**:

1. **M0 (Scaffolding, parallel API)**: introduce all new APIs under new names (`pairee.file`, `pairee.fs.read_dir`, `pairee.cha`, `Command`, etc.). Keep the old `pairee.app.*`, `pairee.fs.*` table-based API working as thin wrappers. Emit a `log::warn!` deprecation notice when the old API is invoked.

2. **M5 (Cleanup, single API)**: after one release cycle with both APIs in production, remove the old API in a major version bump. Update `docs/plugin-dev-guide.md` to reference only the new API. Update the `plugin-template` branch in lockstep.

3. **Manifest migration**: bump `min_pairee` in any bundled plugins. No manifest schema change required; existing manifests continue to work because new fields are optional.

4. **Documentation**: update `docs/plugin-dev-guide.md` and `docs/technical/plugin-system-design.md` to reflect the new API. The new `plugin-roadmap.md` (this document) is the migration reference. The Spanish version (`plugin-roadmap-es.md`) is also published for parity.

---

## 8. Acceptance Test Plugins

To validate that the proposed runtime surface is sufficient, the M0–M4 work should include porting at least three acceptance test plugins. The reference implementations at `example/reference/` (an open-source plugin system) provide the source patterns.

1. **`fzf.pairee`** — fuzzy file navigation. Exercises the `Command` builder (with `PIPED` and `INHERIT` stdio), `Child:write_all` / `wait_with_output`, `ui.hide` / `Permit`, `pairee.cx.active.selected`, `pairee.cx.active.current.cwd`, `pairee.emit("cd" | "reveal" | "toggle_all")`. Target: < 100 lines of Lua.

2. **`zoxide.pairee`** — history navigation. Exercises `pairee.ps.sub("cd", ...)`, `pairee.async`, `pairee.target_os/family`, `pairee.emit("cd")`, `ui.hide`, `Command:env`. Target: < 150 lines of Lua.

3. **`code-preview.pairee`** — syntax-highlighted code preview. Exercises `pairee.preview_code` (or shell-out to `pygmentize` / `bat`), `pairee.ui.Line` / `ui.Text` / `ui.Span` with styles, `pairee.th` access.

A successful port of all three confirms the new API is complete and ergonomic. A failure on any one signals a gap to close before the M5 cleanup.

---

## 9. Implementation Roadmap

Total estimated effort: ~10–14 weeks for a single experienced Rust+Lua developer. Phases can run in parallel where dependencies allow.

### M0 — Scaffolding (1 week)

- Add new `PluginRequest` variants: `EmitAction`, `FileCache`, `InputDialog` (replacing stub), `ConfirmDialog` (replacing stub), `WhichPrompt`.
- Implement the new request dispatchers in `process_plugin_requests` (`src/plugin/manager.rs:164-287`).
- Add a deprecation `log::warn!` on the old `Confirm` and `Input` paths (keep the stub but log loudly).
- Wire up `pairee.emit("cd" | "set_focus")` as a proof of concept for `EmitAction`.
- Add `pairee.file_cache`, `pairee.notify` (extended), `pairee.target_os/family`, `pairee.time`, `pairee.hash` (shell out to a small Rust helper for now — full XxHash later).
- **Done when**: a plugin can call `pairee.emit("cd", {"/tmp"})` and the panel navigates.

### M1 — Core Utils + Real Dialogs (1.5 weeks)

- Implement real `pairee.input` (with realtime, debounce, obscure) and `pairee.confirm` (with pos/title/body) by routing to the TUI popups.
- Add `pairee.which`.
- Add `pairee.quote`, `pairee.percent_encode/decode`, `pairee.json_encode/decode`, `pairee.sleep`.
- Add `pairee.uid/gid/user_name/group_name/host_name` (Unix-only; `nil` on Windows).
- Add `pairee.clipboard` with secure-mode gating.
- **Done when**: a plugin can call `pairee.input({title="Name"})` and the user sees a real input dialog with the value returned.

### M2 — Typed UserData Foundation (3 weeks)

- Add the `Url`, `Path`, `Cha`, `File`, `Error` userdata (per §5.A1–A5).
- Add `add_cached_field` shim for memoizing derived fields.
- Add the `Composer` proxy for lazy namespace resolution.
- Replace `FileEntrySnapshot` with the new `File` userdata, keeping the old struct as a deprecated internal type.
- Update `peek` job construction in `src/plugin/registry.rs:104-149` to pass a real `File` userdata with `cha` and `mime`.
- Add `pairee.image.show/precache/info` reusing the existing `image` crate.
- **Done when**: a plugin can call `entry.cha:perm()` and get a Unix permission string; a plugin can call `pairee.image.show(url, rect)` and see the image in the preview pane.

### M3 — Async fs + Command + Sync Context (3 weeks)

- Add the `Runtime` struct and `runtime_scope!` macro (`src/plugin/runtime/runtime.rs` + `src/plugin/macros.rs`).
- Implement the sync vs async plugin dispatch.
- Parse `--- @sync entry` and `--- @sync peek` annotations at load time.
- Add `preload()` and `seek()` routing.
- Migrate `pairee.fs.read/write/exists/stat/list` to `tokio::fs` (non-blocking).
- Add the new `fs.*` operations per §5.B2.
- Add the `Command` builder, `Child`, `Output`, `Status`, `Access`, `Fd` per §5.B3–B6.
- Add `pairee.state` per §4.F4.
- Port `fzf.pairee` and `zoxide.pairee` as acceptance tests.
- **Done when**: a plugin can write `Command("fzf"):arg("-m"):stdin(PIPED):stdout(PIPED):spawn()` and stream input/output; a plugin can call `pairee.cx.active.current.hovered` and read live state.

### M4 — UI Widgets + cx/rt/th/km (3 weeks)

- Add the 24 UI widgets per §5.C1–C11 as userdata.
- Add the `Renderable` enum and the dispatch to ratatui.
- Add `cx`, `rt`, `th`, `km` per §5.D1–D4.
- Add `pairee.preview_code/widget`.
- Port `code-preview.pairee` as the acceptance test.
- **Done when**: a plugin can build `ui.Line("hello"):fg("red"):bold()` and have it render in the preview pane with red bold text.

### M5 — Cleanup (1 week)

- Remove deprecated APIs from M0 (or keep one more release).
- Update `docs/plugin-dev-guide.md` and `docs/technical/plugin-system-design.md` to reflect the new API.
- Update `plugin-template` branch in lockstep.
- Add deprecation warnings to any remaining old-API callers.
- Run a "migration sprint": port all existing published Pairee plugins (git.pairee, fzf.pairee, etc.) to the new API.
- Run a registry-wide `pairee developer validate` to ensure all bundled plugins conform.
- **Done when**: a clean `cargo build` and `cargo test`; no `pairee.fs.list` calls remain in bundled plugins.

---

## Appendix A — Pairee Current → Proposed API Mapping

Cross-reference for the implementation team.

| Pairee (current) | Pairee (proposed) | Notes |
|---|---|---|
| `pairee.app.cwd()` | `pairee.app.cwd()` | Same |
| `pairee.app.cd(path)` | `pairee.app.cd(path)` | Kept for compat; `pairee.emit("cd", {path})` is the unified form |
| `pairee.app.focus()` | `pairee.app.focus()` | Same |
| `pairee.app.set_focus(side)` | `pairee.app.set_focus(side)` | Kept; `pairee.emit("focus", {side})` is the unified form |
| `pairee.app.notify(title, msg, level)` | `pairee.notify({title, content, timeout, level})` | New structured form |
| `pairee.app.confirm(title, msg)` (stub) | `pairee.confirm({pos, title, body})` | Now real |
| `pairee.app.input(title, default)` (stub) | `pairee.input({pos, title, value, obscure, realtime, debounce})` | Now real |
| `pairee.app.hovered()` | `pairee.app.hovered()` → returns `File` userdata | Richer |
| `pairee.fs.read(path)` | `pairee.fs.read(url)` (async) | Now non-blocking |
| `pairee.fs.write(path, data)` | `pairee.fs.write(url, data)` (async) | Now non-blocking |
| `pairee.fs.exists(path)` | `pairee.fs.cha(url)` / `pairee.fs.exists(url)` | New `cha` userdata |
| `pairee.fs.stat(path)` | `pairee.fs.cha(url)` | New `cha` userdata |
| `pairee.fs.list(path)` | `pairee.fs.read_dir(url, opts)` | Now async + options |
| `pairee.fs.spawn(cmd, args)` | `Command("..."):arg{...}:cwd():env():stdin():stdout():stderr():memory():spawn()` | New builder |
| `pairee.fs.spawn_copy_task(from, to)` | `pairee.fs.copy(from, to)` + `pairee.emit("tasks:update_succeed")` | New form |
| `pairee.ui.Paragraph(text)` | `pairee.ui.Paragraph(text)` | Kept; new `ui.Text` preferred |
| `pairee.ui.Gauge(ratio, label)` | `pairee.ui.Gauge():ratio(r):label(span)` | Builder |
| `pairee.ui.List(items)` | `pairee.ui.List({...})` | Builder |
| `pairee.ui.Table(headers, rows)` | `pairee.ui.Table({Row, ...})` | Builder |
| `pairee.ui.Span(text, style)` | `pairee.ui.Span(text):style(s)` | Builder |
| `pairee.ui.Line(spans)` | `pairee.ui.Line({Span, ...})` | Builder |
| (n/a) | `pairee.ui.Style():fg("red"):bold()` | New |
| (n/a) | `pairee.ui.Layout():direction(H):constraints({...}):split(rect)` | New |
| (n/a) | `pairee.ui.{Rect, Pad, Pos, Border, Bar, Clear, Fill, Align, Wrap, Edge, Constraint, Color}` | New |
| `pairee.ps.sub` | `pairee.ps.sub` | Kept |
| `pairee.ps.pub` | `pairee.ps.pub` | Kept |
| `pairee.ps.unsub` | `pairee.ps.unsub` | Kept |
| (n/a) | `pairee.ps.pub_to` / `pairee.ps.sub_remote` / `pairee.ps.unsub_remote` | New, optional cross-instance |
| `pairee.log.info/warn/error/debug` | `pairee.log.*` (or `pairee.dbg/err`) | Kept |
| `pairee.sync(fn)` | `pairee.sync(fn)` (full implementation) | Full sync/async bridge |
| `pairee.settings.*` | `pairee.settings.*` (read-only, from manifest) | Kept |
| (n/a) | `pairee.state` (mutable per plugin) | New |
| `pairee.t(key, vars)` | `pairee.t(key, vars)` | Kept |
| (n/a) | `pairee.emit(action, args)` | New (M0) |
| (n/a) | `pairee.exec(action, args)` | New (M0) |
| (n/a) | `pairee.file_cache(opts)` | New (M0) |
| (n/a) | `pairee.preview_code(opts)` | New (M4) |
| (n/a) | `pairee.preview_widget(opts, widget)` | New (M4) |
| (n/a) | `pairee.which(opts)` | New (M1) |
| (n/a) | `pairee.image.{show,precache,info}` | New (M2) |
| (n/a) | `pairee.clipboard(text?)` | New (M1) |
| (n/a) | `pairee.quote(str, unix?)` | New (M1) |
| (n/a) | `pairee.{sleep, time, hash, target_os, target_family, json_encode, json_decode, percent_encode, percent_decode}` | New (M1) |
| (n/a) | `pairee.{uid, gid, user_name, group_name, host_name}` (Unix-only) | New (M1) |
| (n/a) | `cx`, `rt`, `th`, `km` (sync-context only) | New (M3/M4) |
| (n/a) | `pairee.image.show` | New (M2) |

---

## Appendix B — Pairee Files Affected

A consolidated list of every Pairee file that needs modification across the M0–M5 plan. Counts are approximate.

### New files (per phase)

**M0** (4 new files):
- `src/plugin/runtime/bindings/emit.rs`
- `src/plugin/runtime/bindings/utils_basic.rs` (file_cache, target_os, time, hash)
- `src/plugin/runtime/bindings/notify_ext.rs`
- `src/plugin/runtime/bindings/which.rs` (stub — full impl in M1)

**M1** (3 new files):
- `src/plugin/runtime/bindings/dialogs.rs` (input, confirm)
- `src/plugin/runtime/bindings/clipboard.rs`
- `src/plugin/runtime/bindings/utils_ext.rs` (quote, sleep, percent, json, uid/gid, etc.)

**M2** (8 new files):
- `src/plugin/types/mod.rs`
- `src/plugin/types/url.rs`
- `src/plugin/types/path.rs`
- `src/plugin/types/cha.rs`
- `src/plugin/types/file.rs`
- `src/plugin/types/error.rs`
- `src/plugin/runtime/bindings/image.rs`
- `src/plugin/runtime/bindings/traits.rs` (add_cached_field, Composer)

**M3** (5 new files):
- `src/plugin/runtime/runtime.rs` (Runtime struct)
- `src/plugin/macros.rs` (runtime_scope!)
- `src/plugin/runtime/bindings/process/mod.rs`
- `src/plugin/runtime/bindings/process/command.rs`
- `src/plugin/runtime/bindings/process/child.rs` (with `access.rs` and `fd.rs`)

**M4** (10 new files):
- `src/plugin/runtime/bindings/ui/elements/{span,line,text,list,paragraph,table}.rs`
- `src/plugin/runtime/bindings/ui/style.rs`
- `src/plugin/runtime/bindings/ui/layout.rs` (with constraint, rect, pad, pos)
- `src/plugin/runtime/bindings/ui/borders.rs` (border, bar, clear, gauge, fill, align, wrap, edge, color)
- `src/plugin/runtime/bindings/ui/renderable.rs`
- `src/plugin/runtime/bindings/cx.rs`
- `src/plugin/runtime/bindings/rt.rs`
- `src/plugin/runtime/bindings/th.rs`
- `src/plugin/runtime/bindings/km.rs`
- `src/plugin/runtime/bindings/preview.rs` (preview_code, preview_widget)

### Modified files

- `src/plugin/manager.rs` — add 5 new `PluginRequest` variants; add dispatchers in `process_plugin_requests`; fix dialog stubs.
- `src/plugin/loader.rs` — parse `@sync`/`@since` annotations; bump version-aware checks.
- `src/plugin/sandbox.rs` — extend `is_command_safe` and `validate_path` for new APIs.
- `src/plugin/registry.rs` — add `Preload`/`Seek` variants; replace table with `File` userdata in `Peek`.
- `src/plugin/runtime/standard.rs` — register new globals (`pairee.state`, `pairee.image.*`, `pairee.which`, etc.).
- `src/plugin/runtime/bindings/app.rs` — reimplement `cd`/`set_focus` as thin wrappers over `EmitAction`; remove `confirm`/`input` stubs (moved to dialogs.rs).
- `src/plugin/runtime/bindings/fs.rs` — migrate to `tokio::fs`; add new operations; add `(value, Error?)` returns.
- `src/plugin/runtime/bindings/ui.rs` — deprecate plain-table constructors; route to new widget modules.
- `src/plugin/runtime/bindings/sync.rs` — full bridge implementation (sync/async dual path).
- `src/plugin/hooks.rs` — enrich the event surface.
- `src/app/state/types.rs` — extend `PopupType` with `WhichPrompt` and the new dialog variants.
- `src/ui/popup.rs` (and `src/ui/popup/*.rs`) — add `which` popup, real input/confirm popups.
- `src/keybindings/resolver.rs` — accept and dispatch `EmitAction` requests from plugins.
- `docs/plugin-dev-guide.md` and `docs/plugin-dev-guide-es.md` — rewrite API surface section.
- `docs/technical/plugin-system-design.md` and `docs/technical/plugin-system-design-es.md` — update to match the new model.
- The `plugin-template` orphan git branch — update boilerplate to the new API.

### Reference acceptance plugin files (new in M3/M4)

- `plugins_dev_dir/fzf.pairee/manifest.toml`, `main.lua`, `lang/en.toml`, `help/en.md`
- `plugins_dev_dir/zoxide.pairee/manifest.toml`, `main.lua`, `lang/en.toml`, `help/en.md`
- `plugins_dev_dir/code-preview.pairee/manifest.toml`, `main.lua`, `lang/en.toml`, `help/en.md`

---

## Appendix C — Reference Material

A vendored third-party TUI file manager with a mature Lua plugin system was used to validate the design and source concrete patterns for the more advanced features (typed userdata, sync/async bridge, widget builder, command streaming, VFS layer). It is vendored at `example/reference/` (gitignored) for local reference and is not part of the Pairee release. No name, source, or attribution is exposed in Pairee's public API or in this document.

The Pairee plugin system is, and will remain, an original design. Pairee's existing strengths — the developer tooling (`pairee developer` CLI, the TUI wizard, the `plugin-template` branch), the manifest-based `settings_schema` rendered in the TUI, the per-plugin i18n with fallback, the F1 help integration, the strict Secure Mode, and the registry with SHA-256 verification — are not derived from any third-party system and are the foundation on which the new runtime surface will be built.

---

*Prepared by the Pairee maintainers as an internal design document. All `file:line` references are accurate as of Pairee 0.6.1 (current `main`). No external names are referenced in this document or in the proposed Pairee API.*
