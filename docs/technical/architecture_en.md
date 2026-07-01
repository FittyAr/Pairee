# Pairee Developer & Architecture Manual

This document details the software design, structure, runtime workflows, and code patterns utilized within the **Pairee** terminal file manager.

---

## 🏛️ 1. Core Architecture & Decoupled State

Pairee is built on the core principle of **separating core application logic from the presentation (UI) layer**. 

```mermaid
graph TD
    subgraph Core Engine
        A[main.rs Entry] --> B[AppConfig Loader]
        B --> C[AppContext Settings]
        C --> D[AppState Data]
        D --> E[fs::ops System Operations]
        D --> F[fs::ops_worker Async Tasks]
    end
    subgraph Event Resolver
        G[crossterm Events] --> H[keybindings::Resolver]
        H --> I[Action Enum]
        I --> J[app::state::handle_action]
    end
    subgraph UI Render Layer
        J --> K[ui::layout Draw Frame]
        K --> L[Ratatui Terminal Backend]
    end
```

### 1.1 Decoupled Terminal State
The core business logic does not import `ratatui` or handle console outputs directly.
* All directory listings, glob filtering, active operations state, selected files lists, and background tasks channels are housed inside `AppState` (`src/app/state/mod.rs`) and `AppContext` (`src/app/context.rs`).
* This enables writing standard Rust unit tests for directory changes, sorting options, and path manipulations without mocking terminal devices.

### 1.2 The Event Loop (`app::run`)
The main execution sequence:
1. `main.rs` builds `AppContext` and `AppState`.
2. `app::run()` starts terminal raw mode using `terminal::backend`.
3. An asynchronous loop listens for terminal resize and key inputs via `terminal::events`.
4. Resolved inputs mutate state parameters and trigger corresponding filesystem changes.
5. The TUI drawing layer renders the modified status on every loop tick.

---

## ⌨️ 2. Keybinding Resolution Engine

Pairee supports custom presets (`norton`, `vim`, `modern`) without bloating UI components with key listener logic.

### 2.1 Event Flow
Keyboard event processing follows a strict unidirectional flow:
1. `crossterm::event::KeyEvent` is captured by the background event producer.
2. The key event is sent to the resolver: `keybindings::resolver::resolve(key, active_preset)`.
3. The resolver returns a logical `keybindings::actions::Action` variant.
4. The action is handled by the application state handler: `app::state::handle_action(action)`.

```rust
// Logical mapping example from keybindings/resolver.rs
pub fn resolve(key: KeyEvent, preset: &str) -> Option<Action> {
    match preset {
        "vim" => resolve_vim_preset(key),
        "norton" => resolve_norton_preset(key),
        _ => resolve_modern_preset(key),
    }
}
```

---

## 🔄 3. Asynchronous Operations & Worker Pattern

For long-running disk operations (Copy, Move, Wipe, Delete), blocking the main rendering loop causes the UI to freeze. Pairee solves this by delegating heavy disk tasks to a background thread pool managed by `tokio`.

```mermaid
sequenceDiagram
    participant UI as UI Loop / AppState
    participant Worker as fs::ops_worker (Tokio Task)
    participant OS as Local File System

    UI->>Worker: Spawn Async Task (paths, destination)
    Note over Worker: Runs on Tokio thread pool
    loop Copying Files
        Worker->>OS: Copy chunk/file
        Worker->>UI: Send progress via crossbeam Channel (e.g. 15% complete, file_x.txt)
        UI->>UI: AppState updates progress metrics and draws progress dialog
    end
    Worker->>UI: Task Complete / Close Channel
    UI->>OS: Refresh active panel listings
    UI->>UI: Close progress popup dialog
```

### 3.1 Progress Channel Lifecycle
* **Task Spawning:** When `Action::Copy` is resolved, `fs::ops_worker::spawn_copy_task` is triggered.
* **Worker Execution:** A background thread handles file enumeration, path checks, read/write loops, and system calls.
* **Progress Reporting:** The worker sends `CopyProgress` updates via a channel sender. The structure reports:
  ```rust
  pub struct CopyProgress {
      pub current_file: String,
      pub files_copied: usize,
      pub total_files: usize,
      pub bytes_copied: u64,
      pub total_bytes: u64,
  }
  ```
* **UI Redraw:** On each tick, the main UI thread drains outstanding updates from the channel receiver into the state variables. If a background operation is active, `ui::popup::prompts::render_prompt_popup` displays a dynamic progress bar widget.

---

## 🌐 4. Centralized Localization & Translations

Translations are handled systematically to prevent code redundancy and hardcoded UI string issues.
* **English Strings:** All default English UI text labels are defined centrally in the embedded [en.toml](file:///d:/GitHub/NCRust/lang/en.toml) file, resolved via `get_default_english_translation(key)`.
* **External & Embedded Translations:** Non-English languages (like Spanish) are compiled directly into the binary ([es.toml](file:///d:/GitHub/NCRust/lang/es.toml)) for portability, and can be overridden dynamically by external TOML files in the `lang/` directory at startup.
* **Translation Helper:** Code files utilize `t("translation_key")` to resolve messages. If a localized file is missing, the engine falls back to default English definitions.

---

## 🖥️ 5. Standalone Terminal Launcher

To support launching Pairee as a desktop app without an open parent terminal session:
* On startup, `main.rs` invokes `terminal::standalone::check_and_launch_standalone()`.
* **Windows Behavior:** The program detects if it was launched from explorer (no parent console attached). If so, it invokes a new shell wrapper (e.g., `cmd.exe` or `powershell.exe`) with the necessary window parameters, hosting the Pairee executable.
* **Linux/macOS Behavior:** Spins up a default system terminal emulator (e.g., `xterm`, `gnome-terminal`, `kitty`) to launch the application.

---

## 🎨 6. Theme Engine & Colors

* Themes are styled via individual TOML profile documents.
* Themes map logical UI elements (e.g. `panel_border`, `file_executable`, `menu_selected`) to terminal-friendly color palettes (e.g. `Color::Blue`, `Color::Rgb(r,g,b)`).
* `ui::theme_apply::parse_color` interprets the TOML strings, translating them into `ratatui::style::Color` rules applied directly during frame draws.

---

## 🔄 7. Auto-Update System

Pairee features a non-blocking, smart software update mechanism designed to query, download, verify, and apply releases across multiple distribution methods and host platforms.

### 7.1 Module Architecture (`src/update/`)
The module is decomposed into focused subcomponents:
* **Installation Method Detector (`detect.rs`):** Determines the method used to install Pairee (e.g., native Linux package managers, Windows installers, manual zip, or manual tarball extract) out of 13 supported profiles.
* **GitHub Release Checker (`checker.rs`):** Queries the GitHub Releases API asynchronously, matching version tags against the current build using semantic versioning (`semver`). A local file-based cache (`update_cache.json`) expires after 1 hour to prevent hitting API rate limits.
* **Streaming Downloader (`downloader.rs`):** Performs segment-by-segment streaming downloads of release assets with live progress report callbacks. It secures the download by computing the SHA-256 hash of the received file and comparing it against the remote release's `.sha256` sidecar asset.
* **Installer Execution Engine (`installer.rs`):** Applies the downloaded update. It performs an atomic binary swap for manual Linux installs, executes Windows Inno Setup packages silently, or creates a self-cleaning helper batch script to replace manual Windows ZIP binaries. For package-manager-tracked environments, it renders copy-to-clipboard terminal update commands.

### 7.2 Event Flow & Background Tasks
1. **Startup Check:** If `auto_update_check` is enabled, a Tokio background worker is spawned at boot to query release APIs.
2. **Visual Notification:** When a new version is detected, a yellow `▲ UPDATE` indicator badge is drawn at the top-right header of the application frame.
3. **Interactive Menu/Popup:** Selecting the update indicator or choosing "Check for updates" in the Options menu opens a dedicated Ratatui popup. It presents the release notes (changelog), version differences, and three choices: "Install Now", "Ignore Version" (updates settings to skip this version tag), or "Close".
4. **Live Progress:** Choosing install starts a background download task. The popup displays a live progress gauge showing byte transfer rates. Once completed, it prompts the user to restart the application.

