# Changelog

All notable changes to Pairee will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/):

- `Added` for new features.
- `Changed` for changes in existing functionality.
- `Deprecated` for soon-to-be removed features.
- `Removed` for now removed features.
- `Fixed` for any bug fixes.
- `Improved` for performance or UX improvements.

## [Unreleased]

### Added

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
