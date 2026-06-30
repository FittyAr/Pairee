# Pairee Plugin System — Technical Design Reference

> **This document describes the planned Lua-based plugin architecture for Pairee. It is a forward-looking technical specification; no code implementing this system exists yet.**

---

## Table of Contents

1. [Overview](#1-overview)
2. [Architecture Diagram](#2-architecture-diagram)
3. [Module Layout](#3-module-layout)
4. [Design Patterns](#4-design-patterns)
5. [Dynamic Keybindings Overlay](#5-dynamic-keybindings-overlay)
6. [Plugin Types & Hooks](#6-plugin-types--hooks)
7. [API Surface Reference](#7-api-surface-reference)
8. [Concurrency & State Model](#8-concurrency--state-model)
9. [Debugging and Logging Engine](#9-debugging-and-logging-engine)
10. [Sandboxing, Trusted Mode, and Secure Mode Protection](#10-sandboxing-trusted-mode-and-secure-mode-protection)
11. [Plugin Localization (I18n)](#11-plugin-localization-i18n)
12. [Plugin Custom Settings](#12-plugin-custom-settings)
13. [Integrated F1 Help Documentation](#13-integrated-f1-help-documentation)
14. [Developer Mode Tools & Pull Request Submission TUI](#14-developer-mode-tools--pull-request-submission-tui)
15. [Registry, Directory Layout, and CLI Command Flow](#15-registry-directory-layout-and-cli-command-flow)
16. [Implementation Milestones](#16-implementation-milestones)

---

## 1. Overview

Pairee's plugin system allows community developers to extend the file manager's behavior without touching the Rust core. Plugins are written in **Lua** and loaded at startup by a `PluginManager` component.

**Key design decisions:**

- **`mlua`** (vendored LuaJIT or Lua 5.4) is the embedding library. It integrates natively with Tokio for async execution.
- Plugin code **never blocks the rendering thread**. All plugin execution runs inside dedicated Tokio tasks.
- The Rust `AppState` is **never directly mutated** by plugins. Mutations flow through a typed event queue.
- Plugin access to dangerous Lua standard libraries (`io`, `os`, `package`) is **blocked by default** and requires explicit user opt-in (`trusted = true`) in `pairee.toml`.
- A global, read-only configuration parameter **`secure_mode`** prevents all internet access and external command execution regardless of individual plugin trust settings.
- Gated behind a `[developer]` setting, a built-in suite handles syntax validation, file auto-formatting, hash calculation for all folder assets, and Git branch/PR automation.

---

## 2. Architecture Diagram

```mermaid
graph TD
    subgraph "Configuration Layer"
        TOML[pairee.toml\n[settings], [plugins], and [developer] tables] --> PM
        LOCK[plugins.lock\nhash mappings + version pins] --> PM
    end

    subgraph "Plugin Manager — src/plugin/"
        PM[PluginManager\nmod.rs] --> LOADER[loader.rs]
        PM --> REGISTRY[registry.rs]
        PM --> SANDBOX[sandbox.rs]
        PM --> DEV[developer_tool.rs]
        PM --> UPDATER[updater.rs]
    end

    subgraph "Lua Runtime — src/plugin/runtime/"
        LOADER --> LUA[mlua::Lua instance\nstandard.rs]
        LUA --> BIND_APP[pairee.app]
        LUA --> BIND_FS[pairee.fs]
        LUA --> BIND_UI[pairee.ui]
        LUA --> BIND_PS[pairee.ps]
        LUA --> BIND_LOG[pairee.log]
        LUA --> BIND_SYNC[pairee.sync]
    end

    subgraph "Plugin Code — ~/.config/pairee/plugins/"
        LOADER --> P1[git.pairee/main.lua]
        LOADER --> P2[fzf.pairee/main.lua]
        LOADER --> P3[custom/main.lua]
    end

    subgraph "App Integration"
        BIND_APP <-->|"AppEvent queue (tokio::mpsc)"| APPSTATE[AppState]
        BIND_SYNC -->|"Snapshot channel (oneshot)"| APPSTATE
        APPSTATE --> HOOKBUS[HookBus\nhooks.rs]
        HOOKBUS --> P1
        HOOKBUS --> P2
    end

    subgraph "Ratatui Render Layer"
        APPSTATE --> DRAW[ui::mod.rs]
        BIND_UI --> DRAW
    end
```

---

## 3. Module Layout

```text
src/
└── plugin/
    ├── mod.rs              # Public API — re-exports, PluginManager::init()
    ├── manager.rs          # Discovers plugins, calls loader, configures hooks
    ├── loader.rs           # Discovers directories, verifies folder files, executes main.lua
    ├── registry.rs         # Name → PluginHandle index
    ├── hooks.rs            # HookBus: subscribe/emit lifecycle events
    ├── sandbox.rs          # Per-plugin Lua stdlib access control & Secure Mode
    ├── developer_tool.rs   # Formatting, validating, hashing, and Git PR Client
    ├── updater.rs          # Registry file-by-file fetch, search, verify, update
    └── runtime/
        ├── mod.rs
        ├── standard.rs     # Lua VM init: creates globals, loads preset Lua
        ├── bindings/
        │   ├── mod.rs
        │   ├── app.rs      # pairee.app.* implementation
        │   ├── fs.rs       # pairee.fs.* implementation
        │   ├── ui.rs       # pairee.ui.* implementation
        │   ├── ps.rs       # pairee.ps.* pub/sub implementation
        │   ├── log.rs      # pairee.log.* debugging implementation
        │   └── sync.rs     # State synchronization implementation
        └── types/
            ├── mod.rs
            ├── entry.rs    # FileEntry type bridged to Lua
            └── job.rs      # PreviewJob / HookEvent context type
```

Each file in `src/plugin/` has a single responsibility, honoring the SRP rules in AGENTS.md.

---

## 4. Design Patterns

* **Strategy Pattern — Plugin Types as Dispatch Keys:** Plugins implement specific roles (`Previewer`, `Preloader`, `Hook`, `Command`) by defining corresponding methods in their returned Lua tables.
* **Observer Pattern — HookBus:** A central `HookBus` holds event subscriptions and notifies subscribers asynchronously through Tokio tasks.
* **Command Pattern — Functional Plugins:** Plugins map names to `entry()` functions which are registered in the global execution registry.
* **Facade Pattern — `pairee.*` Globals:** The `pairee.app`, `pairee.fs`, `pairee.ui`, `pairee.log` tables act as a facade over Rust internals, exposing a clean API.
* **Flyweight / Snapshot Pattern — State Reads:** Plugins retrieve read-only state snapshots (`pairee.sync`) and write mutations through a typed event channel.

---

## 5. Dynamic Keybindings Overlay

To make plugin installation seamless, plugins define default keymaps inside their `manifest.toml` file:

```toml
# In plugin's manifest.toml
[keybindings]
"ctrl+h" = "entry"          # Binds Ctrl+H to this plugin's entry() function
"g"      = "run_action"     # Maps key "g" to the run_action() method
```

During startup, the `PluginManager` reads these keybindings and overlays them on top of Pairee's active key resolver. This guarantees that user shortcuts are configured automatically without manual changes to the core project settings.

---

## 6. Plugin Types & Hooks

### Previewers
Called when a file is highlighted in the active panel. The plugin renders content into the preview pane.

```lua
local M = {}

function M:peek(job)
    local content = pairee.fs.read(tostring(job.file.url))
    return pairee.ui.Paragraph(content)
end

return M
```

### Lifecycle Hooks
React to app navigation events without rendering output.

```lua
local M = {}

function M:setup(opts)
    pairee.ps.sub("on_cd", function()
        local cwd = pairee.sync(function() return pairee.app.cwd() end)
        pairee.log.info("Changed dir to: " .. cwd)
    end)
end

return M
```

### Functional Commands
Invoked explicitly via keybinding or user menus.

```lua
local M = {}

function M:entry(args)
    local result = pairee.fs.spawn("fzf", { "--height=40%" })
    if result.status == 0 and result.stdout ~= "" then
        pairee.app.cd(result.stdout:gsub("\n$", ""))
    end
end

return M
```

---

## 7. API Surface Reference

### `pairee.app`

| Function | Returns | Description |
|----------|---------|-------------|
| `cwd()` | `string` | Active panel's current directory |
| `cd(path)` | — | Navigate to `path` |
| `focus()` | `"left"\|"right"` | Currently focused panel |
| `set_focus(side)` | — | Switch focused panel |
| `notify(title, msg, level)` | — | Show popup. Level: `"info"`, `"warn"`, `"error"` |
| `confirm(title, msg)` | `boolean` | Blocking confirmation dialog |
| `input(title, default)` | `string` | Blocking input dialog |
| `hovered()` | `Entry` | Currently hovered file entry |

### `pairee.fs`

| Function | Returns | Description |
|----------|---------|-------------|
| `read(path)` | `string` | Read file contents |
| `write(path, data)` | — | Write data to file |
| `exists(path)` | `boolean` | Check if path exists |
| `stat(path)` | `Entry` | File metadata |
| `list(path)` | `Entry[]` | List directory entries |
| `spawn(cmd, args)` | `Output` | Run external command |
| `spawn_copy_task(from, to)` | — | Async copy with progress popup |

### `pairee.ui`

| Constructor | Description |
|-------------|-------------|
| `Paragraph(text)` | Plain text block widget |
| `Gauge(ratio, label)` | Progress bar |
| `List(items[])` | Selectable list |
| `Table(headers[], rows[][])` | Grid table |
| `Span(text, style)` | Styled text span |
| `Line(spans[])` | Horizontal span row |

### `pairee.ps`

| Function | Description |
|----------|-------------|
| `sub(event, fn)` | Subscribe to named event |
| `pub(event, data)` | Publish event to all subscribers |
| `unsub(event)` | Remove current plugin's subscription |

### `pairee.log`

| Function | Description |
|----------|-------------|
| `info(msg)` | Logs info level message to centralized system |
| `warn(msg)` | Logs warning level message |
| `error(msg)` | Logs error level message |
| `debug(msg)` | Logs debug level message |

---

## 8. Concurrency & State Model

```
Main thread (60 fps render loop)
    │
    ├── [tick] process AppEvent queue
    │       ├─ AppEvent::ShowNotification → update AppState.notification
    │       ├─ AppEvent::NavigateTo(path) → update AppState.active_panel.cwd
    │
    ├── [tick] HookBus::emit("on_cd", payload)
    │       └─ tokio::spawn → plugin async task
    │               └─ pairee.sync(fn) → sends snapshot request
    │                       └─ main thread replies via oneshot channel
    │
    └── [tick] Ratatui draw frame (reads AppState read-only)
```

---

## 9. Debugging and Logging Engine

To allow robust plugin development, Pairee provides a unified error handling and console output architecture:

1. **Integrated Logs:** The `pairee.log` module routes messages directly to Pairee's log pipeline. The logs are captured and written to `~/.cache/pairee/app.log`.
2. **Error Isolation:** If a plugin encounters a runtime exception (e.g. nil references, dividing by zero, type errors), it is captured at the `mlua` border in Rust. The event loop is unaffected. Pairee displays a visual notification detailing the error and records the backtrace.
3. **CLI Debug Mode:** Developers can run:
   ```bash
   pairee --plugin-debug <plugin-name>
   ```
   This flag configures the logging system to dump all runtime errors and `pairee.log` calls from `<plugin-name>` directly onto stdout, facilitating real-time monitoring.

---

## 10. Sandboxing, Trusted Mode, and Secure Mode Protection

Pairee implements a multi-layered, strict sandboxing and trust boundary to protect user environments from malicious or poorly written plugins.

### 10.1 The Lua Sandboxed VM (Untrusted Mode by Default)
When a plugin runs, its Lua environment is loaded into an isolated VM instance. By default, plugins are restricted to **Untrusted Mode** (`trusted = false`):

1. **Standard Library Filtering:** Only safe core libraries are loaded into the VM:
   * **Allowed:** `base` (excluding dangerous functions), `table`, `string`, `math`, `utf8`.
   * **Omitted and Blocked:** `io` (file operations), `os` (system environment, shell commands), `package` (module resolution mechanics), `coroutine`, and `debug` (VM introspection).
2. **Global Function Isolation:** Standard global functions that allow dynamic or arbitrary execution of code or file integration are stripped or disabled:
   * **Disabled:** `load`, `loadstring`, `dofile`, `loadfile`.
   * **Overridden `require`:** The global `require` function is custom-implemented by the Rust loader. It only allows importing relative Lua submodules located strictly inside the plugin's own installed folder, preventing access to system-wide scripts or modules.
3. **Execution Restrictions:** Any attempt to call `pairee.fs.spawn()` to run external processes immediately results in a runtime script error.

### 10.2 Trusted Mode (`trusted = true`)
For advanced plugins that need system integration (e.g., executing Git or running diagnostic tools), the user can opt-in to **Trusted Mode** in `pairee.toml`:
* The VM is initialized with access to dangerous standard libraries (`io`, `os`, `package`) to allow loading external files and modules.
* The plugin is permitted to invoke external processes using the `pairee.fs.spawn()` API.

### 10.3 Global Secure Mode (`secure_mode = true`)
To prevent even trusted plugins from stealthily exfiltrating data or stealing sensitive files, Pairee implements a global, immutable **Secure Mode** acting as an engine-level safeguard:
* **Activation:** Enabled via the main configuration file (`pairee.toml` under `[settings]` -> `secure_mode = true`).
* **Immutability:** The main configuration object is loaded as read-only by the Rust core; Lua scripts have no write access to this memory space and cannot disable Secure Mode.
* **Network & Socket Interception:** Even if a plugin is set to `trusted = true`, the Rust runtime blocks any TCP/UDP socket creation or HTTP calls from within the VM environment.
* **Process Spawning Blacklist:** The Rust process executor intercepts all calls to `pairee.fs.spawn()`. If the binary matches any prohibited system tool or shell, execution is blocked immediately:
  * **Networking & Egress Tools:** `curl`, `wget`, `nc`, `netcat`, `ssh`, `scp`, `sftp`, `telnet`, `ftp`, `rsync`, `nmap`.
  * **Shells & Command Interpreters:** `sh`, `bash`, `zsh`, `csh`, `tcsh`, `powershell`, `pwsh`, `cmd`, `cmd.exe`.
  * **Script Runtimes & Interpreters:** `python`, `python3`, `perl`, `ruby`, `node`, `php`, `lua`, `luajit`.
* **FS Boundary Sandboxing:** Under Secure Mode, file read/write operations through `pairee.fs` are restricted to the active workspace directory and the user's configuration folders, preventing access to root paths or system directories.

---

---

## 11. Plugin Localization (I18n)

To prevent third-party plugins from cluttering or requiring edits to Pairee's central localization codebase (`src/config/localization/`), plugins package their own translations using isolated TOML files:

### 11.1 File Structure
Each plugin maintains a `lang/` directory containing language files (named after their ISO 639-1 code):
```text
~/.config/pairee/plugins/git.pairee/
├── manifest.toml
└── lang/
    ├── en.toml        # Default language translations
    └── es.toml        # Spanish translations
```

### 11.2 Resolution and Fallback Engine
When a translation is requested from the plugin, the translation engine uses the following resolution steps:
1. **User Locale:** Queries the active system locale configured in Pairee (e.g., `es`). If `lang/es.toml` exists, it looks up the key.
2. **Default Language Fallback:** If the key is missing in the user's locale, or the file `es.toml` is absent, the engine falls back to the plugin's `default_language` declared in `manifest.toml` (e.g., `lang/en.toml`).
3. **Key Identifier Fallback:** If the key is not found in the fallback file, it returns the raw key identifier wrapped in brackets (e.g., `"[messages.git_error]"`). This avoids empty UI text and indicates missing translation keys immediately during execution.

### 11.3 Translation Variable Interpolation
The global translation binding `pairee.t("key", { var = value })` parses strings and performs inline variables replacement (e.g., replacing `{status}` with the value passed in variables table).

---

## 12. Plugin Custom Settings

Pairee allows plugins to declare custom configuration parameters that are integrated automatically into the application's configuration user interface.

### 12.1 Settings Schema Declaration (`manifest.toml`)
Plugins declare their configurable properties under a `[settings_schema]` table in the manifest. Each setting specifies its name, type (`bool`, `string`, or `integer`), default value, and user-facing description:
```toml
[settings_schema]
show_hidden = { type = "bool", default = false, description = "Show hidden VCS files" }
git_path    = { type = "string", default = "git", description = "Custom Git executable path" }
max_depth   = { type = "integer", default = 3, description = "Max directory recursion depth" }
```

### 12.2 Config TUI and Dynamic Rendering
1. **Parsing:** At initialization, Pairee's config loader reads the `[settings_schema]` of all active plugins.
2. **Settings Menu:** The TUI configuration screen renders a dedicated "Plugins Settings" section containing subcategories for each active plugin.
3. **Form Fields:** Selecting a plugin dynamically draws settings input forms:
   * `bool` types render as toggle checkboxes.
   * `string` types render as text entry inputs.
   * `integer` types render as number adjusters.
   Description strings are displayed directly below each control as hover/focused helper text.

### 12.3 Value Persistence (`pairee.toml`)
All customized settings values are stored and maintained inside the user's global `pairee.toml` configuration file under a dedicated `[plugins.settings.<plugin_name>]` table:
```toml
[plugins.settings.git-status]
show_hidden = true
git_path    = "/usr/local/bin/git"
max_depth   = 5
```

### 12.4 Lua API Access (`pairee.settings`)
The resolved settings values are loaded into the plugin's VM context as a read-only global table named `pairee.settings`. The script reads settings keys directly:
```lua
local is_hidden_visible = pairee.settings.show_hidden -- Returns true or false
local cmd = pairee.settings.git_path -- Returns "/usr/local/bin/git"
```

---

## 13. Integrated F1 Help Documentation

Plugins can provide user documentation that is automatically parsed and embedded within Pairee's main offline help system.

### 13.1 Structured Help Files
To keep the plugin workspace organized, help documentation files must reside inside a dedicated `help/` subdirectory, named after their ISO 639-1 language code (with `.md` extension):
```text
~/.config/pairee/plugins/git.pairee/
├── manifest.toml
└── help/
    ├── en.md          # Default English help documentation
    └── es.md          # Spanish help documentation
```

### 13.2 Help UI Rendering (F1 Key)
1. **Pane Integration:** When a user presses `F1` to open the system help, the TUI displays a tabbed layout: "Core Help" and "Plugins Help".
2. **Documentation Listing:** Under "Plugins Help", the system lists all active plugins containing valid help files inside their `help/` directories.
3. **Markdown Rendering:** Selecting a plugin parses its markdown content (e.g. `help/en.md`) and renders it in a scrollable viewer panel, adapting heading colors and block formatting to the user's active TUI theme.

### 13.3 Document Fallback Resolution
The help parser queries the active system locale to select the correct documentation file:
* Checks for `help/<locale>.md` (e.g. `help/es.md`). If present, it loads it.
* Falls back to `help/<default_language>.md` (e.g. `help/en.md`) configured under `default_language` in `manifest.toml` if the localized file is absent.

---

## 14. Developer Mode Tools & Pull Request Submission TUI

When a developer sets `developer_mode = true` in `pairee.toml` under a `[developer]` settings section, a suite of development utilities is unlocked:

### 14.1 CLI Developer Commands & Strict Validations
* `pairee developer format <path>`: Formats all Lua files in the plugin directory according to Pairee's style standards.
* `pairee developer validate <path>`: Runs a syntax, schema, and directory structure linter over all plugin assets, enforcing strict cross-platform checks.
* `pairee developer package <path>`: Scans the directory structure, executes the validation suite, and packages the plugin:
  * **Language Detection:** Scans the `lang/` folder for `*.toml` files, extracts the language codes (e.g., `en`, `es`), and writes them to the `languages` array in `manifest.toml`.
  * **Type Detection:** Parses `main.lua` and checks which hooks or APIs are referenced (e.g., presence of `peek`/`seek` designates a `"previewer"`; `pairee.ps.sub` subscriptions designate a `"hook"`). Writes this to `type` in the manifest.
  * **Integrity Hash Generation:** Automatically generates/updates the `sha256.sum` listing the hashes of every single file within the plugin directory.

### 14.2 Strict Validation Suite Rules
The `validate` and `package` commands (along with the CI runner `./scripts/validate-plugin.sh`) strictly enforce:
1. **Cross-Platform Naming Safety:**
   * Directory and file names must consist exclusively of lowercase alphanumeric characters, dots (`.`), dashes (`-`), and underscores (`_`).
   * No spaces, capital letters, or special characters (such as `?`, `*`, `:`, `\`, `/`, `|`, `<`, `>`, `"`) are permitted.
2. **Name Consistency:**
   * The plugin's root directory name must match exactly the `name` field declared inside `manifest.toml`.
3. **Locale File Coverage:**
   * Every locale declared in `languages` under `manifest.toml` must have both a corresponding translation file (`lang/<locale>.toml`) and help file (`help/<locale>.md`).
   * No extra/untracked translation or help files may exist in those directories without being listed in the `languages` manifest array.
   * A translation file and help file must be present for the specified `default_language`.
4. **Translation Key Synchronization:**
   * Parses all TOML files in `lang/` and verifies that all translation keys are identical across all files. Any missing keys in secondary languages generate warnings.
5. **UTF-8 Encoding:**
   * All `.toml`, `.md`, and `.lua` files must be valid UTF-8.

### 14.3 Automated PR Submission UI & Metadata Wizard
An interactive developer-only TUI metadata builder and submission wizard is added to submit plugins directly from Pairee:
1. **Metadata Wizard:** Prompts the developer step-by-step to input missing manifest fields (author, description, license), displays the auto-detected language files and plugin category, and validates the configuration.
2. **PR Automation:** Collects GitHub PATs, fork URLs, and commit descriptions, packages the files (updating `sha256.sum`), creates a local Git branch, commits, pushes to the fork, and creates the Pull Request on GitHub automatically.

---

## 15. Registry, Directory Layout, and CLI Command Flow

To simplify distribution, the registry distributes raw directory structures with hash checks:

* **Directory format:** Plugins are stored as raw folders in the registry branch under `registry/plugins/<name>/<version>/`.
* **Integrity verification:** A `sha256.sum` file inside the version folder lists the SHA-256 hash of every single file. The installer downloads each file individually and validates its hash.
* **Plugin Management:**
  * `pairee plugin search <query>`: Local search of cached `index.toml`. Results display colored badges for plugin category (e.g., `[Hook]`) and supported languages (e.g., `[EN] [ES]`).
  * `pairee plugin list`: Inspect installed versions, trust settings, and list available updates.
  * `pairee plugin check-updates`: Query the registry branch to check for newer compatible versions.
  * `pairee plugin update`: Automated file-by-file update and hash verification.

---

## 16. Implementation Milestones

| Milestone | Deliverables |
|-----------|-------------|
| **M1 — Engine Foundation** | `mlua` dependency, `src/plugin/` skeleton, `PluginManager::init()`, sandbox |
| **M2 — Core API Bindings** | `pairee.app`, `pairee.fs`, `pairee.ui`, `pairee.ps`, `pairee.log` |
| **M3 — Hook System** | `HookBus`, `on_cd`, `on_hover`, `on_key` integration |
| **M4 — Previewer Support** | Lua `peek()` / `seek()` routing through preview pane |
| **M5 — Keybinding Overlays** | Dynamic merge of shortcuts from `manifest.toml` |
| **M6 — Plugin Localization** | Isolated `lang/*.toml` file loading, fallback logic, `pairee.t()` bindings |
| **M7 — Plugin Settings** | manifest `[settings_schema]` parsing, config UI rendering, `pairee.settings` binding |
| **M8 — Integrated F1 Help** | `HELP.md` / `HELP.locale.md` markdown parsing and `F1` pane integration |
| **M9 — Developer Tools** | `pairee developer` CLI commands, auto-detection logic, PR TUI wizard |
| **M10 — Registry CLI** | `pairee plugin` search/list (with badges), install, update, check-updates |
| **M11 — Registry Branch** | `plugin-registry` branch setup, directory layout, index, CI validation |
| **M12 — Community Docs** | Full public documentation published |

---

*See also: [plugin-registry-spec.md](plugin-registry-spec.md) for the registry branch layout and submission workflow.*
*See also: [plugin-dev-guide.md](../plugin-dev-guide.md) for how to write and submit a plugin.*
