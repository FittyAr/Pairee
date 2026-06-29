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

### Changed

- Moved `manifests/winget/README.md` to `docs/README.md` and corrected its relative links.
- Updated release version bumping scripts (`bump_version.ps1` and `bump_version.sh`) to automatically migrate and update WinGet manifests in the version-specific `manifests/f/FittyAr/Pairee/<version>` directories.
- Updated local references inside `docs/winget-submission-guide.md` to match the versioned folder structure under `manifests/f/`.
- Updated MSIX package manifest (`manifests/msix/AppxManifest.xml`) and documentation to target `Windows.Desktop` rather than `Windows.Universal` for proper desktop environment compatibility and corrected default version settings.
- Made the release workflow's version replacement regex robust in `.github/workflows/release.yml` to prevent build failures during version increments.
- Replaced hardcoded Microsoft Store publisher credentials in `manifests/msix/AppxManifest.xml` with template placeholders to avoid hardcoding sensitive publisher data in the repository.
- Updated `.github/workflows/release.yml` to dynamically inject actual Partner Center identity values from GitHub Secrets or Variables, with a fallback chain targeting local developer defaults.
- Updated local scripts (`run.bat` and `run.sh`) to automatically substitute placeholders with local developer testing defaults before packaging.

### Removed

- Removed the obsolete and unused `manifests/winget/` directory.
- Packaged `docs/` folder from being distributed in release assets, installers, packages, and staging configurations.

### Fixed

- Resolved startup crash (`STATUS_DLL_NOT_FOUND` / `0xC0000135`) on clean Windows installations by ensuring the CI release pipeline statically links the C runtime library.
- Fixed GitHub Actions release workflow dirty cache issues by adding a clean step to force static CRT compilation.

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
