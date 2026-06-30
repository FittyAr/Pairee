# Implementation Plan — Pairee Plugin System

This document outlines the detailed architectural changes and code files needed to implement the Lua-based plugin system for **Pairee**.

---

## User Review Required

> [!IMPORTANT]
> The implementation adds a new scripting runtime dependency (`mlua` with Lua 5.4 vendored). This compiles Lua as part of the Rust executable, meaning no external Lua installation is required on the user's system.
> The implementation strictly respects the security boundary defined by **Untrusted Mode** (default sandboxing), **Trusted Mode** (explicit opt-in in `pairee.toml`), and **Global Secure Mode** (immutable network block and binary blacklist).

---

## Goals

| # | Goal | Description |
|---|------|-------------|
| **G1** | **Lua Runtime Embedding** | Integrate `mlua` to load and execute Lua modules from the config plugins directory. |
| **G2** | **Zero Main-Thread Blocking** | Run all plugin execution inside background Tokio tasks, using thread-safe communication. |
| **G3** | **Sandboxing & Secure Mode** | Restrict dangerous libraries (io, os, package) by default. Block process spawning of shell/network tools in Secure Mode. |
| **G4** | **API Bindings (`pairee.*`)** | Bridge app context, filesystem, UI widgets (for previews), logs, and pub/sub events. |
| **G5** | **Dynamic Keybinding Overlays** | Merge shortcuts declared in plugin manifests dynamically into the key resolver. |
| **G6** | **Localization & Settings** | Support plugin-specific settings schemas and translation fallbacks (`lang/*.toml`). |
| **G7** | **Integrated Help (F1)** | Parse and render plugin `help/<locale>.md` files in a dedicated "Plugins Help" tab. |
| **G8** | **CLI Developer Tools & PR Wizard** | Formatter, structure linter, hash builder, and Git PR builder TUI. |
| **G9** | **Plugin Registry CLI** | Download, file-by-file verification via SHA-256 against `plugins.lock`. |

---

## Proposed Changes

### 1. Project Dependencies

#### [MODIFY] [Cargo.toml](file:///home/fitty/GitHub/Pairee/Cargo.toml)
* Add `mlua` dependency:
```toml
mlua = { version = "0.9.9", features = ["lua54", "vendored", "serialize"] }
```

---

### 2. State & UI Integration

#### [MODIFY] [types.rs](file:///home/fitty/GitHub/Pairee/src/app/state/types.rs)
* Add a `plugin_widget` field to `PopupType::QuickViewPanel`:
```rust
    QuickViewPanel {
        path: PathBuf,
        content: Vec<String>,
        scroll: usize,
        image_data: Option<image::DynamicImage>,
        plugin_widget: Option<crate::plugin::runtime::types::PluginWidget>,
    },
```
* Add a `Help` variant extension to support tabs:
```rust
    Help {
        mode: usize, // 0 = list focus, 1 = reader focus
        docs: Vec<(String, PathBuf)>, // Core docs
        plugin_docs: Vec<(String, PathBuf)>, // Plugin docs
        active_tab: usize, // 0 = Core Help, 1 = Plugins Help
        cursor_idx: usize,
        scroll_y: usize,
        active_content: Option<String>,
    },
```

#### [MODIFY] [settings.rs](file:///home/fitty/GitHub/Pairee/src/config/settings.rs)
* Define configuration structures for plugins:
```rust
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PluginConfig {
    pub name: String,
    #[serde(default)]
    pub trusted: bool,
}
```
* Add `plugins` map and `plugin_settings` table to the `Settings` struct:
```rust
    #[serde(default)]
    pub plugins: std::collections::HashMap<String, PluginConfig>,
    #[serde(default)]
    pub plugin_settings: std::collections::HashMap<String, std::collections::HashMap<String, String>>,
```

---

### 3. Core Plugin System (`src/plugin/`)

We will create a structured hierarchy inside `src/plugin/` to implement all requirements, observing the Single Responsibility Principle.

#### [NEW] [mod.rs](file:///home/fitty/GitHub/Pairee/src/plugin/mod.rs)
* Entry point of the plugin module. Re-exports initialization and event hooks.

#### [NEW] [manager.rs](file:///home/fitty/GitHub/Pairee/src/plugin/manager.rs)
* Discovers and manages plugin lifetimes.
* Handles configuration, loads `plugins.lock`, runs startup execution.
* Integrates `process_plugin_requests` to run synchronous state-read functions on the main thread.

#### [NEW] [loader.rs](file:///home/fitty/GitHub/Pairee/src/plugin/loader.rs)
* Reads the plugin directory, verifies dependencies and version compatibility (`min_pairee`), and executes `main.lua` in the sandboxed VM.

#### [NEW] [registry.rs](file:///home/fitty/GitHub/Pairee/src/plugin/registry.rs)
* Keeps track of loaded plugins, their registered previewers, functional commands, and active keybindings.

#### [NEW] [hooks.rs](file:///home/fitty/GitHub/Pairee/src/plugin/hooks.rs)
* Implements the HookBus: handles async event emission (`on_cd`, `on_hover`, `on_key`, `on_focus`) and manages subscriptions.

#### [NEW] [sandbox.rs](file:///home/fitty/GitHub/Pairee/src/plugin/sandbox.rs)
* Implements sandboxed VM restriction rules.
* Filters Lua standard libraries.
* Provides custom implementation of global `require` wrapper to isolate directory traversal.
* Blocks exfiltration, network socket creation, and executing blacklisted processes in Secure Mode.

#### [NEW] [runtime/mod.rs](file:///home/fitty/GitHub/Pairee/src/plugin/runtime/mod.rs)
* Initializes standard tables and configures environment variables.

#### [NEW] [runtime/standard.rs](file:///home/fitty/GitHub/Pairee/src/plugin/runtime/standard.rs)
* Initializes the Lua state, loading standard utilities and binding the core `pairee` global namespace.

#### [NEW] [runtime/bindings/app.rs](file:///home/fitty/GitHub/Pairee/src/plugin/runtime/bindings/app.rs)
* Implements `pairee.app.*` (focus, cd, cwd, notify, confirm, input, hovered). Uses channels to communicate blocking UI queries (confirm, input) to the main thread.

#### [NEW] [runtime/bindings/fs.rs](file:///home/fitty/GitHub/Pairee/src/plugin/runtime/bindings/fs.rs)
* Implements `pairee.fs.*` (read, write, exists, stat, list, spawn, spawn_copy_task). Spawns commands securely and respects sandboxing constraints.

#### [NEW] [runtime/bindings/ui.rs](file:///home/fitty/GitHub/Pairee/src/plugin/runtime/bindings/ui.rs)
* Exposes UI constructors (Paragraph, Gauge, List, Table, Span, Line) returning structured tables representing widgets.

#### [NEW] [runtime/bindings/ps.rs](file:///home/fitty/GitHub/Pairee/src/plugin/runtime/bindings/ps.rs)
* Exposes `pairee.ps.*` for pub/sub channel communication. Serializes messages across isolated plugin VMs using JSON.

#### [NEW] [runtime/bindings/log.rs](file:///home/fitty/GitHub/Pairee/src/plugin/runtime/bindings/log.rs)
* Binds `pairee.log` calls to the Rust logger.

#### [NEW] [runtime/bindings/sync.rs](file:///home/fitty/GitHub/Pairee/src/plugin/runtime/bindings/sync.rs)
* Implements `pairee.sync()` wrapper logic, sending closures to be evaluated on the main thread event loop where state queries are safe.

#### [NEW] [runtime/types/mod.rs](file:///home/fitty/GitHub/Pairee/src/plugin/runtime/types/mod.rs)
* Exposes Rust bridging types.

#### [NEW] [runtime/types/entry.rs](file:///home/fitty/GitHub/Pairee/src/plugin/runtime/types/entry.rs)
* Bridge for file entry metadata representation in Lua.

#### [NEW] [runtime/types/job.rs](file:///home/fitty/GitHub/Pairee/src/plugin/runtime/types/job.rs)
* Exposes structured widget structures:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PluginWidget {
    Paragraph(String),
    Gauge { ratio: f64, label: String },
    List(Vec<String>),
    Table { headers: Vec<String>, rows: Vec<Vec<String>> },
    Span { text: String, style: String },
    Line(Vec<PluginWidget>),
}
```

#### [NEW] [updater.rs](file:///home/fitty/GitHub/Pairee/src/plugin/updater.rs)
* Handles download, list, updates, and removal. Download checks files individually, calculating SHA-256 hashes and validating them against `plugins.lock`.

#### [NEW] [developer_tool.rs](file:///home/fitty/GitHub/Pairee/src/plugin/developer_tool.rs)
* Implements formatting, validation, checksum generation (`sha256.sum` and type auto-detection), and automated GitHub PR submission logic.

---

### 4. CLI & Event Loop Integration

#### [MODIFY] [main.rs](file:///home/fitty/GitHub/Pairee/src/main.rs)
* Initialize `PluginManager` at startup.
* Route CLI commands `pairee plugin <subcommand>` and `pairee developer <subcommand>` to their respective modules, bypassing GUI startup.
* Look for `--plugin-debug <name>` to enable real-time debugging output.

#### [MODIFY] [app/mod.rs](file:///home/fitty/GitHub/Pairee/src/app/app/mod.rs)
* In the main event loop, call `plugin::manager::process_plugin_requests` to handle state synchronization requests from background plugin threads.

#### [MODIFY] [events.rs](file:///home/fitty/GitHub/Pairee/src/app/app/events.rs)
* Intercept key events and check for active plugin keybindings before running default action resolvers.
* Emit key press events to the HookBus (`on_key`).

#### [MODIFY] [navigation.rs](file:///home/fitty/GitHub/Pairee/src/app/actions/navigation.rs)
* Emit directory change events (`on_cd`) and hover events (`on_hover`) to the HookBus.

---

### 5. UI Renderers

#### [MODIFY] [quickview.rs](file:///home/fitty/GitHub/Pairee/src/ui/quickview.rs)
* If `plugin_widget` is defined inside `QuickViewPanel`, render the corresponding Paragraph, Gauge, List, or Table widget.

#### [MODIFY] [help.rs](file:///home/fitty/GitHub/Pairee/src/ui/popup/prompts/help.rs)
* Support the tabbed layout. Draw a "Core Help" and "Plugins Help" tab. Show the appropriate document list depending on the selected tab.

---

## Verification Plan

### Automated Tests
* Create unit tests in `src/plugin/sandbox.rs` to verify that blocked libraries cannot be accessed in Untrusted Mode.
* Verify that file operations fail outside the sandboxed workspace in Secure Mode.
* Run `cargo test` to execute all tests.

### Manual Verification
* Deploy a test plugin (e.g., `hello` and `csv-preview`) in `~/.config/pairee/plugins/`.
* Test opening help (`F1`) to see the tabbed plugins view.
* Run `pairee plugin search` and check the CLI output.
* Run `pairee developer validate` on a sample plugin.
