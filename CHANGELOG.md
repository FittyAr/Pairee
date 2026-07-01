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

- A dedicated `plugin-template` git branch (orphan) containing the canonical boilerplate files for new plugins (`manifest.toml`, `main.lua`, `lang/en.toml`, `help/en.md`, `icon.png`, `screenshots/screenshot1.png`). The template is never surfaced in any plugin list in the UI.
- New `clone_from_template()` function in the developer tool that uses the `git2` crate to extract template files directly from the local `plugin-template` branch without requiring an external `git` binary. Placeholder tokens (`PLUGIN_NAME`, `PLUGIN_DESCRIPTION`, `PLUGIN_AUTHOR`) are substituted after extraction.
- `PAIREE_REPO_DIR` environment variable support allowing developers to explicitly point to the Pairee source repository for template resolution.

### Fixed

- Plugin initialization via the Developer Tools wizard no longer produces empty `main.lua` and `manifest.toml` files. Files are now sourced from the `plugin-template` branch, with a graceful fallback to the previous localization-string method if the branch is unavailable.

- Default manifest templates updated to include `icon` and `screenshots` variables with helpful instructions.
- A step-by-step interactive assistant wizard (both in TUI and CLI) when initializing new plugin skeletons, prompting for the plugin's name, description, and author.
- Mandatory submission validation checks enforcing the presence of an `icon.png` file (recommended size 512x512 or 256x256 pixels) and a `screenshots/` directory containing at least one image file (PNG/JPG/JPEG) to allow publishing.
- Updated developer guides (English and Spanish) to document the mandatory icon and screenshot publishing requirements.
- Enforced the presence of the `default_language` parameter in the plugin manifest during developer tool lint checks.
- A new interactive TUI Plugins Manager popup (F11) featuring a tabbed split interface to view installed plugins (with trust/pin badges and update indicators), show detailed metadata descriptions, search the online registry repository with live query editing, toggle trust and pinned statuses, and perform background installs/updates or immediate uninstalls.
- Integrated a new interactive TUI **Developer Tools** tab (Tab 2) under the F11 Plugins Manager (accessible when `plugins_developer_mode` is enabled). It features an interactive TUI console to initialize plugin skeletons, run security/compliance lint audits, package files with SHA-256 hashes, and format submission logs.
- Added a `plugins_developer_mode` option in the Language & Plugins configuration screen (Tab 4) to toggle developer-centric TUI helpers.
- Added a customizable `plugins_dev_dir` setting to allow configuring the plugins development directory, with dynamic row rendering in Tab 4.
- Enabled scanning the `plugins_dev_dir` path to perform Lint and Package audits recursively across all subfolders containing `manifest.toml` without requiring installation.
- Automated GitHub submission inside the TUI by prompting the user for their Personal Access Token and initiating forks asynchronously in the background.
- Added an **active development plugin** selection system to the Developer Tools tab. A new option (0) allows selecting a specific plugin from the development directory as the active target. All development operations (Lint, Package, Install, Submit) now operate exclusively on the selected active plugin. The "Initialize new plugin" option is automatically disabled while another plugin is active. The active plugin selection persists across sessions and can be quickly deselected with Backspace/Delete/d from the selector or by entering an empty name.
- Added support for local development plugin installation via the `Shift+F11` (`InstDev`) key legend shortcut, copying plugin files from the highlighted panel directory directly to local plugins for testing.
- Dynamic population of plugin help documentation files (`help/<locale>.md` or default language fallback) inside the tabbed F1 Help menu.
- `check-updates` and `update` subcommands to the `pairee plugin` CLI suite to query and apply plugin updates.
- DeepWiki project documentation link and badge to the main README files.
- Comprehensive plugin system architectural plan covering engine design, design patterns (Strategy, Observer, Command, Facade, Snapshot), API surface (`pairee.app`, `pairee.fs`, `pairee.ui`, `pairee.ps`, `pairee.sync`, `pairee.log`), concurrency model, dynamic keybinding overlays, logging/debugging tools, trusted mode permissions, a global immutable secure mode parameter, isolated plugin localization (`lang/*.toml`) with fallback rules, dynamic custom settings schemas with persistent storage, and F1 Help structured markdown documentation (`help/*.md`) integration.
- Plugin community registry design: versioned distribution of raw plugin folders via a `plugin-registry` GitHub branch with individual file SHA-256 integrity verification, `plugins.lock`, and CLI commands (`pairee plugin search/list/install/update/check-updates/pin/remove`) featuring visual category and language badges, and help file tracking.
- Technical documentation: `docs/technical/plugin-system-design.md` and `docs/technical/plugin-system-design-es.md` — full architecture reference in English and Spanish.
- Technical documentation: `docs/technical/plugin-registry-spec.md` — registry branch layout, manifest/index TOML schemas, CLI reference, download verification flow, versioning rules, and CI workflow plan.
- Developer guide: `docs/plugin-dev-guide.md` and `docs/plugin-dev-guide-es.md` — step-by-step guide for writing, testing, and submitting Pairee plugins, including translation, packaging, the developer assistant TUI wizard, and strict directory/naming validations.

### Improved

- Upgraded the plugin packaging developer tool workflow (Option 3). It now fetches/clones the central `plugin-registry` branch locally, validates the plugin, copies files to the registry branch, and updates the local `index.toml` catalog file.
- Upgraded the plugin submission developer tool workflow (Option 5) in both TUI and CLI. It now prompts the developer for a commit/PR description, commits all changes locally, and either automates the fork/push/PR creation on GitHub (if a token is provided) or displays clear manual instructions on how to manually fork, push, and open the Pull Request (if no token is provided).
- Updated local help pages (`help/es/user_guide.md`, `help/en/user_guide.md`) and development guides (`docs/plugin-dev-guide.md`, `docs/plugin-dev-guide-es.md`) to document these changes.

### Changed

- Replaced the asymmetric hybrid translation model (compiled Rust match blocks for English and JSON for Spanish) with a symmetric, portable TOML-based translation system using embedded files (`lang/en.toml` and `lang/es.toml`) with support for dynamic local directory overrides.
- Replaced hardcoded help files documentation layout with dynamic locale directory scanning (e.g. `help/en/`, `help/es/`) and auto-detection, utilizing translated metadata keys in localizations.
- Resolves active language code for plugin help documentation files dynamically to match the program's running locale.
- Dynamically resolve active language codes for plugins based on the application's loaded language configuration.
- Localized all developer console output messages inside `src/plugin/developer_tool.rs`.
- Refactored `list` command of the `pairee plugin` CLI suite to be asynchronous and check/flag available updates dynamically.
- Updated `README.md` and `README.es.md` with a clean, structured design, a modern features list, and a dedicated project status section inspired by the Yazi layout.
- Moved `manifests/winget/README.md` to `docs/README.md` and corrected its relative links.
- Updated release version bumping scripts (`bump_version.ps1` and `bump_version.sh`) to automatically migrate and update WinGet manifests in the version-specific `manifests/f/FittyAr/Pairee/<version>` directories.
- Updated local references inside `docs/winget-submission-guide.md` to match the versioned folder structure under `manifests/f/`.
- Updated MSIX package manifest (`manifests/msix/AppxManifest.xml`) and documentation to target `Windows.Desktop` rather than `Windows.Universal` for proper desktop environment compatibility and corrected default version settings.
- Made the release workflow's version replacement regex robust in `.github/workflows/release.yml` to prevent build failures during version increments.
- Replaced hardcoded Microsoft Store publisher credentials in `manifests/msix/AppxManifest.xml` with template placeholders to avoid hardcoding sensitive publisher data in the repository.
- Updated `.github/workflows/release.yml` to dynamically inject actual Partner Center identity values from GitHub Secrets or Variables, with a fallback chain targeting local developer defaults.
- Updated local scripts (`run.bat` and `run.sh`) to automatically substitute placeholders with local developer testing defaults before packaging.
- Updated the plugin initialization wizard to automatically create the expected `help/` and `screenshots/` directories, along with placeholder files/images (`icon.png`, `screenshots/screenshot1.png`, `help/en.md`, and optionally `help/es.md`).

### Removed

- Removed the obsolete and unused `manifests/winget/` directory.
- Removed the obsolete `en.rs` translation file and inlined the English fallback resolver inside `localization.rs`.
- Packaged `docs/` folder from being distributed in release assets, installers, packages, and staging configurations.

### Fixed

- Fixed the plugin initialization wizard failing to initialize a skeleton when the `plugin-template` branch is not available locally (e.g. when running the installed binary) by implementing a robust built-in fallback template generator.
- Fixed a TUI crash (exit status 101) when displaying developer console text with accented characters by refactoring string wrapping to be UTF-8 safe.
- Fixed the TUI submit command path resolving to look for plugins inside the developer directory instead of the active process directory.
- Fixed a potential panic in the developer lint tool when validating the default language of a plugin.
- Fixed the plugin packaging and local installation logic to recursively traverse, hash, and copy all plugin subdirectories and files, ensuring folders like `help/` and `screenshots/` are included.
- Fixed terminal stdout corruption and screen distortion when initializing a plugin skeleton in the Developer Tools tab by preventing print statements from executing in TUI mode.
- Fixed manifest.toml deserialization to support both flat formats and nested `[plugin]` table formats, resolving installation failure for initialized plugins.
- Fixed text wrapping and border overflow issues in the Plugins Manager console details panel, and ensured the popup area is fully cleared on redraw.
- Extracted and localized default plugin boilerplate template files, showcasing how to use `pairee.t` localization and lang files inside initialized plugins.
- Updated packager and local sync installation processes to dynamically scan and hash all files in the plugin directory instead of using a hardcoded list.
- Enforced plugin folder naming convention ending in `.pairee` across initialization, loaders, linter, packager, installation, and verification steps.
- Updated the Plugins Manager TUI list display to strip the `.pairee` suffix, showing only the clean plugin name.
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
