# Pairee Plugin Developer Guide

> **This guide explains how to write, test, and submit a Lua plugin for Pairee.**

---

## Table of Contents

1. [Overview](#1-overview)
2. [Plugin Types](#2-plugin-types)
3. [Your First Plugin — Hello World](#3-your-first-plugin--hello-world)
4. [Plugin File Structure](#4-plugin-file-structure)
5. [API Reference Summary](#5-api-reference-summary)
6. [Dynamic Keybindings Overlay](#6-dynamic-keybindings-overlay)
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
| **Previewer** | `peek(job)`, `seek(job)` | A file is highlighted in the active panel |
| **Preloader** | `preload(job)` | A file is about to come into view |
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
min_pairee = "0.7.0"
```

Register it in `pairee.toml`:

```toml
[plugins.hello]
name    = "hello"
trusted = false
```

Launch Pairee, open the command input (or bind it to a key), and execute `plugin:hello`. A notification will appear.

---

## 4. Plugin File Structure

A plugin is a folder containing a main Lua script, manifest, and optional submodules or resources:

```text
~/.config/pairee/plugins/
└── my-plugin/
    ├── main.lua              # Required — plugin entry point
    ├── manifest.toml         # Required — metadata and keybindings
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
pairee.app.cwd()                        -- string: current directory
pairee.app.cd(path)                     -- navigate to path
pairee.app.focus()                      -- "left" | "right"
pairee.app.set_focus(side)              -- switch panel
pairee.app.notify(title, msg, level)   -- show popup ("info","warn","error")
pairee.app.confirm(title, msg)          -- boolean: confirmation dialog
pairee.app.input(title, default)        -- string: text input dialog
pairee.app.hovered()                    -- Entry: currently hovered file
```

### `pairee.fs` — Filesystem & Processes

```lua
pairee.fs.read(path)                    -- string: file contents
pairee.fs.write(path, data)            -- write data to file
pairee.fs.exists(path)                 -- boolean
pairee.fs.stat(path)                   -- Entry: file metadata
pairee.fs.list(path)                   -- Entry[]: directory listing
pairee.fs.spawn(cmd, args)             -- Output: {stdout, stderr, status}
pairee.fs.spawn_copy_task(from, to)    -- background copy with progress popup
```

### `pairee.ui` — Widget Constructors

```lua
pairee.ui.Paragraph(text)
pairee.ui.Gauge(ratio, label)           -- ratio: 0.0 to 1.0
pairee.ui.List(items)                   -- items: string[]
pairee.ui.Table(headers, rows)
pairee.ui.Span(text, style)
pairee.ui.Line(spans)
```

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

## 6. Dynamic Keybindings Overlay

Instead of requiring users to manually modify their global settings files, a plugin can declare its default shortcuts directly inside its `manifest.toml`:

```toml
# In manifest.toml
[keybindings]
"ctrl+h" = "entry"          # Binds Ctrl+H to this plugin's entry() function
"g"      = "run_action"     # Maps key "g" to the run_action function
```

When the plugin is loaded, Pairee's keybinding resolver automatically merges these shortcuts into the runtime environment. If the user uninstalls the plugin, the keybindings are cleanly removed.

---

## 7. Writing a Previewer Plugin

Previewers implement `peek(job)` and optionally `seek(job)` for scrolling.

The `job` parameter provides:
- `job.file` — the hovered `Entry` (with `.url`, `.mime`, `.size`, etc.)
- `job.area` — the available preview area dimensions
- `job.skip` — current scroll offset

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
    return pairee.ui.Table(headers, rows)
end

return M
```

Register as a previewer for CSV files in `pairee.toml`:

```toml
[[previewers]]
mime = "text/csv"
plugin = "csv-preview"
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

Command plugins implement `entry(args)`. They are invoked explicitly via keybinding or command line using `plugin:<name>`.

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
* **Require Isolation:** The global `require` function is custom-implemented by Rust to restrict module loading exclusively to files within the plugin's own directory.
* **No Commands:** Any call to `pairee.fs.spawn()` will immediately throw a runtime error.

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
* The plugin can use Lua's standard file and module management (`io`, `os`, `package`).
* The plugin can execute external system commands using the `pairee.fs.spawn()` API.

### 13.3 Global Secure Mode (`secure_mode = true`)
To prevent data exfiltration, users can enable a global **Secure Mode** in their settings. This mode acts as an overriding runtime firewall:
* **Activation:** Configured in `pairee.toml` under `[settings]` -> `secure_mode = true`. This configuration is read-only at runtime and completely immutable from Lua.
* **Network & Socket Ban:** TCP/UDP socket creation and HTTP requests are completely disabled at the engine level for all plugins, including those with `trusted = true`.
* **Process Spawning Blacklist:** The process execution engine blocks attempts to run any command matching network-capable utilities, shells, or scripting environments:
  * **Network Utilities:** `curl`, `wget`, `nc`, `netcat`, `ssh`, `scp`, `sftp`, `telnet`, `ftp`, `rsync`, `nmap`.
  * **Shells & Interpreters:** `sh`, `bash`, `zsh`, `csh`, `tcsh`, `powershell`, `pwsh`, `cmd`, `cmd.exe`.
  * **Script Runtimes:** `python`, `python3`, `perl`, `ruby`, `node`, `php`, `lua`, `luajit`.
* **FS Sandboxing:** File APIs (`pairee.fs`) are locked to the active workspace folder and the user's config directory.

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

### 15.2 Automated PR Submission UI & Metadata Wizard
An interactive developer-only TUI metadata builder and submission wizard is added to submit plugins directly from Pairee:
1. **Metadata Wizard:** Guides the developer step-by-step to fill in missing manifest fields (author, description, license), displays the auto-detected language files and plugin category, and validates the configuration.
2. **PR Automation:** Collects GitHub PATs, fork URLs, and commit descriptions, packages the files (updating `sha256.sum`), creates a local Git branch, commits, pushes to the fork, and creates the Pull Request on GitHub automatically.

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

---

## 16. Registry Submission Flow

1. **Enable Developer Mode** in `pairee.toml`.
2. Open the **Metadata Wizard TUI** or run `pairee developer package ~/.config/pairee/plugins/my-plugin` to validate your files, auto-detect metadata, and generate hashes.
3. Input your GitHub credentials and fork URL into the PR Submission TUI.
4. Execute "Submit Pull Request" to automate committing, pushing, and generating the PR targeting the `plugin-registry` branch.

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

Plugins can declare their own settings and configuration variables. These are dynamically loaded and rendered in Pairee's main TUI options window, allowing users to modify parameters without editing Lua code directly.

### 18.1 Defining settings_schema
In your plugin's `manifest.toml`, define a `[settings_schema]` table containing the configurable options. Each option must specify a `type` (`bool`, `string`, or `integer`), a `default` value, and a brief `description`:
```toml
[settings_schema]
show_hidden = { type = "bool", default = false, description = "Show hidden VCS files" }
git_path    = { type = "string", default = "git", description = "Custom Git executable path" }
max_depth   = { type = "integer", default = 3, description = "Max directory recursion depth" }
```

When users navigate to the options window, Pairee will present a form specific to your plugin displaying these controls. The values chosen by the user are stored persistently in their `pairee.toml` settings file.

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

[keybindings]
"ctrl+h" = "entry"
```

---

## 21. Best Practices & Conventions

| Convention | Guidance |
|------------|----------|
| **Plugin naming** | Use `<name>.pairee` style for registry plugins (e.g. `git.pairee`, `fzf.pairee`) |
| **Namespaced events** | Prefix pub/sub events with your plugin name: `my-plugin:event` |
| **Decline gracefully** | In `peek()`, check the file type and return early if not applicable |
| **Async-first** | Avoid blocking calls. Use `pairee.fs.spawn` and async patterns |
| **Error notifications** | Show `pairee.app.notify` with level `"error"` on failures — never silently fail |
| **Trust transparency** | If your plugin needs `trusted = true`, document it clearly in the manifest |
| **SemVer discipline** | Bump MAJOR only on breaking behavioral changes |
| **Minimal surface** | Expose only what the plugin needs — less surface = less attack surface |

---

*See also: [docs/technical/plugin-system-design.md](technical/plugin-system-design.md) for the Rust engine architecture.*
*See also: [docs/technical/plugin-registry-spec.md](technical/plugin-registry-spec.md) for the registry schema and submission workflow.*
