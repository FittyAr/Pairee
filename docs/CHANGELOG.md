# Changelog

All notable changes to Pairee will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/):

- `Added` for new features.
- `Changed` for changes in existing functionality.
- `Deprecated` for soon-to-be removed features.
- `Removed` for now removed features.
- `Fixed` for any bug fixes.
- `Improved` for performance or UX improvements.

---

## [v0.8.0] - 2026-09-08

### Added

- Which-key overlay (`Ctrl+Shift+K`) lists the live keymap chords with labels, fuzzy-filters them, and runs the selected action with Enter. While a multi-key sequence is in progress, a prefix hint shows the remaining chords (same `keybinds` map, not a second keymap). Esc cancels the prefix.
- Copy path (`Ctrl+Shift+C`, Files menu, command palette) puts the hovered or tagged full path(s) on the OS clipboard. Command palette is bound as `Ctrl+Shift+P` in the shipped keymaps.
- Short threat model (`docs/THREAT_MODEL.md`) for plugins, SSH presets, updates, and the elevated helper.
- Structured tracing architecture (`tracing`, `tracing-subscriber`, `tracing-appender`) with rolling daily logs and environment filter control.
- Cross-platform POSIX and Windows signal handling (`SIGTERM`, `SIGINT`, `SIGHUP`, `SIGQUIT`, and console control events) for clean TUI restoration.
- CI test coverage workflow using `cargo-llvm-cov` to measure and report codebase test coverage.

- Plugin confirm, input, and which-key dialogs are real TUI overlays (`pairee.confirm` / `pairee.input` / `pairee.which`); Enter/Esc (and Y/N) reply to the waiting plugin.
- Typed `File` userdata (`name`, `path`, `url`, `size`, `is_dir`, `is_symlink`) and `pairee.cx` (cwd, hovered, selected) filled inside `pairee.sync`.
- Lua `File` metadata: `mime`, `mtime`, `is_hidden`, `is_exec` (Lua API **1.1.0**).
- Plugin filesystem extras: `mkdir`, `remove`, `rename`, `copy`, `read_dir`, and `file()` (File userdata).
- `pairee.Command` process builder with piped `Child` streaming (`write_all`, `read`, `wait_with_output`).
- Optional feature flags in Settings → System: SSH, plugins, and image preview (Git already had a toggle). Existing configs stay enabled.
- EN/ES translation keys are now complete and checked in CI (`scripts/check_translations.py`, `docs/i18n.md`).
- Parser smoke-fuzz tests (globs, descript.ion, plugin manifests, settings TOML) so junk input cannot panic.
- CI tests run on macOS as well as Linux and Windows.
- First-run keymap onboarding (Norton / Neovim / VS Code). Existing configs skip the dialog.
- Versioned Lua plugin API **v1.1.0** (`pairee._lua_api_version`, `docs/api/lua/`).
- CI acceptance plugins under `tests/plugin_acceptance/` (surface, fs, cx/utils, Command echo).
- `pairee.emit`, `pairee.notify`, and `pairee.file_cache` are callable functions (they were nested tables).
- Improvement tracking document at `docs/IMPROVEMENT_PLAN.md` with phased roadmap and progress checkboxes.
- Integration tests under `tests/` cover isolated temp workspace, Settings TOML roundtrip, shipped keymap presets, packaged EN/ES keys, AppState panel roots, and zip extract.
- Project-level `rustfmt.toml` and `clippy.toml` for consistent CI quality gates.
- Declared MSRV (`rust-version = "1.88"`, required by `tui-scrollbar`) and package metadata in `Cargo.toml`.
- Transfer Strategy backends (`local` / `ssh`) under `src/fs/transfer/backend/` with unified job submission.
- Command palette (`Ctrl+Shift+P`) to filter and run logical actions.
- Fractional scrollbars via `tui-scrollbar` (shared helper in `src/ui/scrollbar.rs`) on help, viewer/quickview, history lists, transfer panel, git panel, and related popups.
- Mouse drag and jump-to-click on scrollbars (`ScrollBarInteraction`, hit targets registered each frame, `EnableMouseCapture`).
- Unicode-aware file-name truncation helpers (`unicode-width` + `unicode-segmentation`) for panel columns.
- Bracketed paste: pasted text lands in the CLI or the open text prompt (rename, apply command, mkdir, …) as one string instead of fake keystrokes.
- Settings → Interface shows keymap validation (errors, warnings, bound count) and a “View keymap issues” overlay. Far-style `Gray+` / `Gray-` / `Gray*` aliases are documented as mapping to `Plus` / `-` / `*`.
- CI draw/resize smoke tests via `ratatui` TestBackend (not a substitute for a human pass on Windows Terminal / conhost / Linux).
- CI `cargo deny` job checks licenses, RustSec advisories, yanked crates, and crate sources (`deny.toml`).

### Improved

- Complete codebase modularization under the Single Responsibility Principle (SRP), bringing all `.rs` files across the project strictly below 300 lines (zero files exceed 300 lines).
- Decoupled `PopupType` variants into dedicated sub-structures (`GitPanelState`, `SshConnectPromptState`, `ConfigurationDialogState`, `PluginMenuState`, `CopyMovePromptState`) to keep overlay payloads modular, lightweight, and maintainable.
- Background Terminal (`command &`) and apply-command run on a real PTY (Windows ConPTY / Unix pty), so programs that check for a TTY can emit colors and use normal line buffering.
- Apply-command (`Ctrl+G`) opens the Terminal screen and shows captured stdout/stderr with ANSI colors, while the Transfer Engine still tracks progress.
- Command palette entries come from a shared `ActionDef` catalogue (`id` + category) instead of a second hardcoded name list.
- Native 7z extract/list uses maintained `sevenz-rust2` instead of unmaintained `sevenz-rust` (Zip-Slip sanitization kept).
- Command palette filters with Helix `nucleo-matcher` (fuzzy ranking) instead of substring `contains`.
- MSRV is **1.93** (required by `sevenz-rust2`).
- Clipboard writes use `arboard` instead of shelling out to `clip` / `xclip` / `wl-copy` (update “copy command” and Copy path).
- Background Terminal screen (`command &`) renders ANSI SGR colors from captured stdout/stderr instead of showing raw escape codes.
- Full-screen terminal clear runs inside the synchronized-update region, and only on resize or after a native TTY/admin restore.
- Keyboard enhancement and focus-change sequences are enabled only when the terminal supports them (Unix query; Windows CSI push), and popped only if they were pushed.
- File-association and CLI command lines split with POSIX `shlex` (quoted words) on Unix, and quote-aware tokens on Windows so `"C:\\Program Files\\App\\app.exe" %f` stays one program name.
- Keybindings engine rebuilt on the `keybinds` crate: invalid chords are rejected, duplicate chords across actions are rejected, and Norton/Neovim/VSCode presets load from validated TOML.
- TUI draw path uses synchronized updates and dirty-flag rendering to reduce flicker/glitches.
- Scroll indicators use theme colors and proportional thumbs instead of ratatui’s full-cell default.
- Clippy collapsible-if and related lint cleanups so `cargo clippy -- -D warnings` is green again.
- Clippy 1.98 cleanups (`useless_borrows_in_formatting`, `question_mark` in Lua `t()` lookup).
- CI `check` workflow now targets `master`/`main`, runs tests on Ubuntu and Windows, uses Node 24-aligned actions, and rejects crate-level `clippy::all` allows.
- Documentation index (`docs/README.md`) lists design docs with Implemented/Partial/Planned status.
- README (EN/ES) links corrected to `help/en` and `help/es`, project tree updated, plugin system no longer labeled as only planned.
- Transfer worker split into focused modules (scan, delete, copy, helpers) under `src/fs/transfer/worker/` using a facade orchestrator.
- Copy, move, and delete (including SSH) now use the Transfer Engine progress UI instead of the legacy modal-only path.
- Wipe, compress, extract, and apply-command jobs use the Transfer Engine queue and minimized panel (one consistent progress UX).
- Cooperative cancel for archive compress/extract (native formats check cancel between entries; external 7z is killed on cancel).

- Session state grouped into `PanelPair`, `HistoryState`, and `UpdateState` on `AppState`.
- Split oversized UI modules (transfer panel, history lists, settings actions, plugin dev options) into focused files.
- Internal F3 viewer split into `src/ui/viewer/{state,text,hex,image}.rs`.
- Overlay `PopupType` paste handling and plugin widgets live in focused files under `src/app/state/popup/`.
- Overlay dialogs live in `src/app/state/popup/` (`QuickViewDialog` boxed; config settings boxed) so `PopupType` is no longer a huge enum payload.
- Plugin updater, directory listing, and Settings split into focused modules.
- Dialogs use a `DialogStack` (`state.dialogs`) with replace/push/pop instead of a single `Option` popup.
- Background channels (search, SSH, terminal, updates, plugin progress) are polled in place instead of take/put-back.

### Changed

- Application logic lives in the `pairee` library crate; `src/main.rs` is a thin tokio entry so tests can `use pairee`.
- Replaced inherited rustc-style `.gitignore` with a Pairee-specific ignore list.
- Plugin manager core module renamed to `lifecycle` to avoid module-inception nesting.
- Long-running file jobs no longer use a separate modal progress dialog.

### Deprecated

### Removed

- Local temporary `.tmp*` workspaces and the vendored local `example/` reference tree from the working tree (still ignored by git).
- Legacy `ops_worker` spawn stack, `progress_rx` / `BackgroundOpContext`, and the `CopyProgress` modal UI.

### Fixed

- Trusted Lua plugins load again (`StdLib::ALL_SAFE` instead of `ALL`, which rejected `debug` under `new_with`).
- Clippy is enforced without `#![allow(clippy::all)]` in `src/main.rs`.
- Outdated status banners on transfer-engine and plugin-system design docs.

---

## [v0.7.2] - 2026-08-06

### Added

- Interactive dialog for file associations enabling navigation, addition, editing, and deletion, with clear visual prompts and helper hints on keys to use.
- Expanded Git support with comprehensive backend APIs for individual file staging, unified diffs, remote syncing (fetch, pull, push), advanced branch management, stashing, resets, merges, and repository clone/initialization.
- New Git dashboard TUI integration with an interactive 4-tab panel (Status, Log, Branches, and Stash).
- Unified diff viewer modal with syntax-colored lines for additions, deletions, and hunks.
- Interactive popup dialogs for stash creation, branch creation/renaming, and safe confirm-action dialogs for resets, merges, and stashes.
- New Spanish translation and updated English manual for Git integration reference.
- New `F7` Rename action that prompts only for the new filename (with a live collision warning if a sibling already exists).
- `Rename` command added to the **Top Menu Bar → Files** submenu.
- F-key shortcut bar now reads each slot from the active keybinding resolver, so the bar always shows what each F-key actually does.
- `Create folder` (MkDir) action added as a default option in the **User Menu** (`F2`), bindable to key `6`. The action opens the same name prompt dialog used everywhere else.

### Improved

- Alt+G Git panel initialization now populates stash data immediately on launch.
- F2-F12 F-key shortcut bar now matches the actual action each key triggers: F2 = User Menu, F9 = Top Menu, F7 = Rename, F11 = empty (when not bound).
- Bottom F-key bar no longer claims `F11 = Plugin` by default — the F11 slot now renders blank until the user explicitly rebinds the key.

### Changed

- Expanded default file association presets to support a wide range of popular formats (text, code, images, audio, video, documents, and web pages).
- F6 dialog renamed from "Rename/Move" to "Move" only — Rename is its own modal now.
- `Make Folder` and `Plugin commands` moved out of the F-key bar into the **Top Menu Bar → Files** submenu so the bar can focus on the most frequent operations.
- The plugin system (`PluginMenu` action) is no longer reachable from `F11`. It is now accessible exclusively via **Top Menu Bar (`F9`) → Files → Plugin commands**. Power users can still rebind `F11` to `plugin_menu` in `keybindings.toml` if they prefer the old layout.

### Removed

- Default keymap no longer binds `F7` to `MkDir` or `F11` to `PluginMenu`. `MkDir` lives in the User Menu (F2) and `PluginMenu` lives under the top menu bar (F9 → Files). Power users can still rebind the keys in `keybindings.toml`.

### Fixed

- Single file copy target path resolution so copying a file to a target destination path no longer creates an extra directory with the file name.
- F-key bar in `keymaps/*.toml` preset files now reflects the new keymap (F7→Rename, no F7→MkDir, no F11→PluginMenu), so users upgrading keep the bar and behavior in sync.
- Outdated doc comment on `Action::MkDir` that still claimed the action was bound to `F7`.

---

## [v0.7.1] - 2026-07-20

### Added

### Improved

### Changed

### Deprecated

### Removed

### Fixed

- Fixed PowerShell command execution syntax in GitHub Actions workflow (`.github/workflows/release.yml`) during MSIX packaging.
- Fixed directory tree removal when moving folders in the background Transfer Engine so that empty source subdirectories and root folders are completely cleaned up.

---

## [v0.7.0] - 2026-07-20

### Added

- High-performance asynchronous Transfer Engine inspired by TeraCopy, enabling non-blocking background file copying, moving, and deletion.
- Redesigned Transfer Panel with a split two-column layout featuring a vertical jobs queue sidebar, detailed job inspector, options controls, speed statistics, and logs.
- Advanced transfer controls including queueing multiple jobs, pause/resume, file skipping, job cancellation, speed throttling, and error handling options (`halt_on_error`).
- Cryptographic hash verification supporting CRC32, MD5, SHA-1, SHA-256, and BLAKE3 algorithms, with automatic HTML and CSV post-transfer report generation.
- Multiplatform post-transfer automated actions: system shutdown, sleep, hibernate, application exit, and drive ejection.
- Interactive file conflict resolution dialog with batch options (Overwrite All, Overwrite Older, Skip All, Rename All) and full path visibility.
- Support for Windows Long Paths (Unicode `\\?\`) in direct I/O operations for filenames exceeding 260 characters.
- Interactive TUI Plugins Manager (`F11`) with tabbed browsing, real-time registry search, background installation, update management, and remote blocklist filtering.
- Dedicated TUI Developer Tools tab under the Plugins Manager featuring an interactive plugin initialization wizard, lint auditing, dynamic packaging, and test harness execution.
- Command-line interface additions for plugin management (`pairee plugin check-updates`, `update`, and multi-plugin installation).
- Persistent transfer folder history (`transfer_history.toml`) saving recent source and destination paths.

### Changed

- Pressing Enter on a file now opens it directly in Pairee's native viewer (text, image, or hex). External editor execution on Enter is now optional via the `enter_use_external` setting.
- Replaced the translation backend with a portable, symmetric TOML translation engine (`lang/en.toml`, `lang/es.toml`) with local override support.

### Removed

- Removed the obsolete horizontal transfer queue tab in favor of the new vertical jobs sidebar.

### Improved

- Improved disk free space checking across platforms before starting file transfers.
- Optimized TUI rendering performance during large-scale file transfers using a sliding display window and log cap.
- Enhanced search experience in the Plugins Manager with instant background filtering as you type and keyboard navigation support.
- Added dynamic color coding in the Transfer Panel to clearly distinguish job statuses (green for completed, yellow for paused, red for cancelled or failed).
- Updated help menu (`F1`) to dynamically load localized documentation files (`help/<locale>.md`).
- Centralized and localized all remaining hardcoded user-facing strings across application dialogs, menus, and editor screens in English and Spanish.

### Fixed

- Resolved application startup crash (`STATUS_DLL_NOT_FOUND`) on clean Windows installations.
- Fixed an issue where cancelling a background transfer job could freeze or leave the engine in an un-restartable state.
- Fixed directory deletion failures caused by leftover empty file description files (`descript.ion`).
- Fixed text entry and `Tab` key focus traps inside the Plugins Manager search input.
- Fixed visual display artifacts and text overflow in the TUI Transfer Panel and Developer Console.
- Fixed plugin packaging validation and skeleton generator fallbacks when operating offline.
- Hardened CLI system execution against command injection vulnerabilities.

---

## [v0.6.1] - 2026-06-27

### Added

- A rule in `.agents/AGENTS.md` to enforce checking and running workspace customization skills automatically.
- WinGet installation helper submenu to `run.bat` and `run.sh` for auto-detecting, forcing architecture installs (x64/arm64), upgrading, and uninstalling Pairee.
- Comprehensive `docs/winget-submission-guide.md` documenting the manual first-time submission, PR troubleshooting, and GitHub Actions release automation.
- Detailed `docs/technical/microsoft-store-publishing.md` explaining how to package and publish Pairee to the Microsoft Store as an MSIX package without a paid certificate.
- Microsoft Store (MSIX) Developer Menu submenu in `run.bat` and `run.sh` for local packaging, test certificate generation, signing, and installation of MSIX packages.
- MSIX manifest template (`AppxManifest.xml`) and asset placeholders under `manifests/msix/`.
- Automatic MSIX packaging and version bumping for Windows targets integrated into the `.github/workflows/release.yml` release workflow.
- Desktop shortcut creation option and Windows Control Panel uninstallation icon support in the Inno Setup installer script.
- Custom Windows resource compilation (`manifests/windows/pairee.rc`) to embed the new multi-resolution icon (`pairee.ico`) directly inside the built `pairee.exe` executable.
- Linux desktop launcher entry (`manifests/linux/pairee.desktop`) and SVG/PNG app icon packaging in `Cargo.toml` for Debian and RPM packages.

### Changed

- Updated local WinGet package manifests for v0.6.0: corrected the license to `GPLv3`, added the `arm64` installer architecture details with valid SHA-256 hashes, and added the Spanish (`es-ES`) translation locale.
- Configured Windows targets (MSVC) in Cargo to statically link the C runtime library (CRT), eliminating runtime dependencies on `VCRUNTIME140.dll`.

### Improved

- Enlarged the self-update popup, added line wrapping for release notes, styled markdown headers, and implemented vertical scrolling with a scrollbar.
- Cached the installation method detection using `OnceLock` to prevent TUI thread freezes during self-update rendering and activation.

### Fixed

- Output duplication in `scripts/extract_changelog.sh` that caused duplicated release descriptions on GitHub releases.
- WinGet validation error (STATUS_DLL_NOT_FOUND) resolved by adding VC++ Redistributable package dependencies in the manifest.
- Syntax errors and rendering bugs in `run.bat` helper script resolved.

---

## [v0.6.0] - 2026-06-26

### Added

- Process name filtering in the task list dialog with live list reordering and deactivated color styling for non-matching entries.
- Interactive "About" dialog (accessible via options menu or shortcut) displaying license, project details, and dependencies with scrolling support.
- Automated version bumping and release note extraction scripts to automate release steps.
- Dynamic build-time metadata tracking (target platform, Git commit hash, build profile) integrated into the binary using a new `build.rs` script.
- Structured GitHub issue templates (bug reports, feature requests) and config templates to standardize community feedback.
- Community health documents including `CODE_OF_CONDUCT.md` and `CONTRIBUTING.md`.
- Workspace customization folder `.agents/` with automated AI skills (`localize-helper`, `settings-helper`, `changelog-helper`) and guidelines `AGENTS.md`.
- Parameterized `Dockerfile.namespace` for containerized compiling and testing in namespace environments.
- GitHub Actions validation workflow (`check.yml`) to automatically test code, formatting, and lints on pull requests.

### Changed

- Decoupled and modularized individual popup prompt rendering logic into dedicated files.
- Consolidated and moved workspace AI instructions from root `agents.md` to `.agents/AGENTS.md`.

### Fixed

- MkDir dialog: typed characters now immediately reflect in the input field.
- Rename/Move dialog: `to` input field is no longer empty and accepts text; first button now shows the correct `Rename` label.
- Copy dialog: destination `to` path is now correctly pre-filled.
- Update popup: `Esc` now dismisses the download progress dialog and no longer locks the UI.

### Improved

- All dialog popups now use fixed-height layouts, preventing input fields, checkboxes, and buttons from being cut off in standard terminal sizes (80×24).

---

## [v0.5.1] - 2026-06-25

### Added

- Cross-platform installation detection: Pairee detects how it was installed (installer, portable, package manager) and issues the appropriate upgrade command.
- Terminal key diagnostics tool for debugging input event handling.
- User menu system: users can now define custom menus with their own commands.
- Expanded panel view modes: additional display options with custom descriptions and file metadata.

### Changed

- Modularized file operation prompt UIs: each prompt type now lives in its own module under `src/ui/popup/prompts/file_ops/`.
- Modularized menu, popup, screen input handler, and main app loop into sub-modules for improved maintainability.
- CI: bumped `actions/checkout` to v7 and `action-gh-release` to v3.

---

## [v0.5.0] - 2026-06-25

### Added

- Automated self-update system: Pairee checks for new releases on GitHub, downloads, and installs updates with a progress UI.
- Comprehensive English and Spanish user documentation covering keyboard shortcuts, SSH/SFTP, Git integration, and configuration.

### Changed

- Filesystem deletion is now recursive with elevated operations support.
- Interactive configuration dialog management added.

---

## [v0.4.1] - 2026-06-24

### Added

- Configuration dialog system with interactive settings management UI.
- User-defined menus with custom shell commands and process restart support.
- Sort mode menu with configurable sort actions.
- Multiple key bindings support for a single custom action (comma-separated).
- Integrated Git workflow: repository status, log viewer, commit management, and dedicated UI panels.

### Changed

- Pre-flight Git authentication checks added to version bump scripts to prevent failed pushes.
- Help documentation UI redesigned with a split-pane layout, scrollbar, and improved keyboard navigation.
- Cross-platform elevated privilege handling refactored for filesystem operations.
- Dependencies bumped to v0.4.0 baseline.

---

## [v0.3.2] - 2026-06-17

### Added

- SSH connection presets with navigation and management support in the connection popup.
- SSH disconnect functionality with a menu option.
- Multiple panel rendering modes for the file explorer.
- Background file copy worker with progress tracking and admin privilege escalation support.

### Fixed

- `ssh2` dependency restricted to correct platform-specific targets; vendored OpenSSL enabled for non-Windows builds.

### Changed

- Filesystem operation error messages are now localized.
- File system operations refactored into dedicated service modules (`delete`, `mkdir`, `rename_move`).
- Resource and localization path resolution enhanced with recursive directory searching.
- Documentation directory added to all installer configurations.

---

## [v0.2.2] - 2026-06-15

### Added

- Filesystem operations with interactive user confirmation dialogs and progress tracking.
- File operation dialogs: rename/move, copy, delete, link, wipe, compress.
- Confirmation dialogs and modular file system operations logic.
- Localization system for all user-facing strings.

---

## [v0.2.1] - 2026-06-15

### Added

- Editor and viewer input handling with file operation integrations.
- Core application state management with modular initialization.
- UI prompts for help, file operations, and localized configuration management.
- System helper module for process management, drive enumeration, bookmarks, and tree navigation.

---

## [v0.2.0] - 2026-06-11

### Added

- GitHub Actions workflow for multi-platform binary releases (Linux GNU, Linux musl, Windows x64/arm64).
- Extensible keybinding resolver and registry system for mapping application commands.
- X11 modifier polling and input handling modules.
- Localization support infrastructure.

---

## [v0.1.7] - 2026-06-11

### Added

- Initial dual-panel TUI file manager core with `ratatui` + `crossterm`.
- Basic directory listing, navigation, and focus management.
- Application event loop with resize handling.
- Configuration loading from TOML files.
- Theme system with color and style definitions.

---

## [v0.1.6] - 2026-06-10

### Fixed

- `cargo-deb` path validation errors.
- Inno Setup `iscc.exe` argument translation on Windows CI runner.
- Cross-compilation pipeline errors for musl targets.

---

## [v0.1.2] - 2026-06-10

### Added

- Automated CI/CD release pipeline with version bumping scripts.
- Inno Setup installer configuration for Windows.
- `cargo-deb` and `cargo-generate-rpm` packaging for Linux.
- Version bump scripts (`bump_version.ps1` / `bump_version.sh`).

---

## [v0.1.1] - 2026-06-10

### Added

- Initial project skeleton with `main.rs` entry point and module layout.
- `Cargo.toml` with core dependencies: `ratatui`, `crossterm`, `tokio`, `serde`, `directories`, `anyhow`, `thiserror`, `log`, `simplelog`.
