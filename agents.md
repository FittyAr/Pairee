# Developer Guidelines for AI Agents (`agents.md`)

This document establishes the architecture, design principles, and guidelines for any AI agent or developer modifying or extending the **NCRust** codebase. All changes must adhere strictly to these rules to maintain high modularity, testability, and cross-platform compatibility.

---

## 1. Core Principles

### Single Responsibility Principle (SRP)
* **Rule:** Each source file (`.rs`) must perform exactly one, well-defined task.
* **Reasoning:** Prevents files from growing into unmaintainable "god files". Makes code review, testing, and AI-driven modifications easy.
* **Example:** Do not combine filesystem scanning and file rendering in the same file. Keep `src/fs/list.rs` and `src/ui/panel.rs` completely separate.

### Zero Hardcoding
* **Rule:** No magic numbers, default file paths, key names, user-facing strings, or color hex codes may be hardcoded in the core application logic.
* **Implementation:** 
  * Static text or layouts must be read from configurations or system defaults.
  * UI themes must be loaded from theme files.
  * Keybindings must go through the keybinding resolver.

### Separation of Core and UI (decoupled state)
* **Rule:** The filesystem layer, configurations, and core event loop must be independent of `ratatui`.
* **Reasoning:** Allows testing the application logic without a terminal backend (e.g., in unit tests). The UI layer should simply read the `AppState` and render it.

### Extensibility (Open/Closed Principle)
* **Rule:** New features (e.g., a new archive format extraction, new search tools, plugins) must be added by implementing traits or adding modules, without modifying existing core structures.
* **Pattern:** Use Rust traits to define plugin interfaces (e.g., a `Viewer` trait for the F3 viewer to support image thumbnails, hex dumps, or text viewing).

---

## 2. Directory & Module Structure

The project follows a strict modular structure. Ensure that new files are placed in their respective modules:

```text
NCRust/
├── Cargo.toml
├── agents.md                      # This file
├── norton_commander_features.md   # Reference for features
└── src/
    ├── main.rs                    # Application entry point
    ├── app/                       # Application loop & state orchestration
    │   ├── mod.rs
    │   ├── app.rs                 # Main application event loop
    │   ├── state.rs               # Application state (panels, focus, dialogs)
    │   └── context.rs             # Runtime context (active config, themes, drives)
    ├── config/                    # Config files management & serialisation
    │   ├── mod.rs                 # Loader/saver configuration API
    │   ├── settings.rs            # Structs for general settings
    │   ├── keybindings.rs         # Structs for keybinding presets
    │   ├── theme.rs               # Structs for modern styling themes
    │   └── paths.rs               # Platform-specific path resolution
    ├── keybindings/               # Keybinding mapping engine
    │   ├── mod.rs
    │   ├── actions.rs             # Logical application Action enum
    │   ├── preset.rs              # Structs for predefined key binding sets
    │   └── resolver.rs            # Mapping crossterm events to logical actions
    ├── fs/                        # Filesystem operations
    │   ├── mod.rs
    │   ├── entry.rs               # File and directory representation
    │   ├── list.rs                # Directory listing & filtering
    │   ├── ops.rs                 # Standard operations (mkdir, delete, rename)
    │   └── ops_worker.rs          # Background jobs (copy/move with progress channels)
    ├── ui/                        # TUI Rendering Components (Ratatui)
    │   ├── mod.rs                 # Main UI draw entry point
    │   ├── layout.rs              # Divides terminal into panels, header, footer
    │   ├── panel.rs               # Renders left/right file lists
    │   ├── cli.rs                 # Renders the command command-line block
    │   ├── menu.rs                # Renders top dropdown navigation
    │   ├── fkeys.rs               # Renders bottom F1-F10 shortcuts
    │   ├── popup.rs               # Dialog windows (prompts, copy progress, errors)
    │   └── theme_apply.rs         # Styles conversion from Config theme to Ratatui
    └── terminal/                  # Raw terminal control & Input listener
        ├── mod.rs
        ├── backend.rs             # Terminal initialization and restoration
        └── events.rs              # Background event producer (keys, resize)
```

---

## 3. Technology Stack & Selected Libraries

Do not implement standard functionality from scratch. Use these pre-selected libraries:

1. **Terminal UI & Drawing:** `ratatui` (Modern Rust TUI framework).
2. **Terminal Control & Events:** `crossterm` (Cross-platform backend for Windows/Linux terminal raw mode, resizing, and keyboard handling).
3. **Serialization:** `serde` with `serde_derive`. For config, use `toml` parsing.
4. **Platform Directories:** `directories` (To correctly locate settings folders like AppData on Windows and `.config` on Linux).
5. **Asynchronous/Concurrency:** `tokio` (For background file operations, keeping the UI responsive during long-running tasks).
6. **Error Handling:** `thiserror` (for custom error enums in modules) and `anyhow` (for application-wide high-level error handling).
7. **Logging:** `log` and `simplelog` (writing debug/error info to an `app.log` file in the user cache directory).

---

## 4. Coding Patterns & Constraints

### Keybinding Resolution Pattern
Keyboard event handling must flow as follows:
```
crossterm::event::KeyEvent 
    ──> [keybindings::resolver::resolve(key, active_preset)] 
    ──> keybindings::actions::Action 
    ──> [app::state::handle_action(action)]
```
*No raw keyboard matching in the UI components.*

### Background Task Pattern
For long-running file system operations (like Copy or Move):
1. **Initiate:** Main thread spawns a Tokio task (`fs::ops_worker::spawn_copy_task`).
2. **Progress:** The task sends progress updates (e.g. `15% completed`, `file_x.txt`) back through a crossbeam/tokio channel.
3. **Render:** The UI reads the active channel progress in `AppState` and displays a modern popup progress bar.
4. **Complete:** When complete, the channel closes, the UI refreshes the directory listing, and the popup closes.
*Never block the main rendering thread.*

### Cross-Platform Path Handling
* Always use `std::path::Path` and `std::path::PathBuf`.
* Never hardcode path separators (use `/` or path methods which resolve automatically on Windows and Linux).
* Use `fs::canonicalize` carefully (behaves differently with UNC paths on Windows).

---

## 5. Verification Requirements

Before submitting code, verify:
1. **Compilation:** `cargo check` and `cargo build` run cleanly on both target systems (or cross-compilation is simulated).
2. **Formatting & Lints:** Code must pass `cargo fmt --all -- --check` and `cargo clippy --all-targets -- -D warnings`.
3. **Testing:** Unit tests must be written for all non-UI components (e.g., config loading, keybinding resolution, path calculations). Run `cargo test`.
