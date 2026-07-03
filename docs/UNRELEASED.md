## [Unreleased]

### Added

- `enter_use_external` setting (default `false`) to manually enable launching external association commands (e.g. `nano %f`) when opening files with Enter. Exposed in the Editor & Viewer settings tab as "Use external command when opening files with Enter". Replaces the previous always-on default behavior.
- Interactive modal selection list for Developer Tools Option 0 (Select Active Plugin), displaying all detected plugins from `plugins_dev_dir` as well as any plugin found in the active Panel 1 or Panel 2 directories, navigable with keyboard arrows and confirmed with Enter.
- Auto-selection of newly created plugins in Option 0 upon successful initialization in Option 1 (which subsequently disables Option 1).
- Remote blocklist support to disable or hide unsafe and broken plugins from search and remote listings.
- Automatic license detection and auto-assignment during packaging, prompting for license names when present but undeclared, or auto-assigning `"MIT"` and generating a standard `LICENSE` file if not present.
- Asynchronous loading of the installed plugin list when opening the Plugin Manager (F11), with a spinner and status text shown in the Installed tab while the registry index is being fetched.
- Live progress bar and status text in the Developer Tools console that stream coarse-grained milestones (e.g. "Cloning registry…", "Copying file 3/12…", "Computing SHA-256…") during long-running operations (init, lint, package, install, submit, plugin scan).
- Three new navigation items in the Developer Tools menu (6, 7, 8) to move the active file panel directly to the dev plugin folder, the packaged plugin folder inside the local registry clone, and the submit folder, closing the popup on success so the developer can inspect the files without copying paths manually.
- Technical documentation: `docs/technical/plugin-roadmap.md` and `docs/technical/plugin-roadmap-es.md` — internal design document enumerating 14 gaps in the current plugin system with file:line evidence and proposing 6 areas of new runtime surface (typed userdata, async fs + Command, rich UI widgets, live context, real dialogs + action dispatch + utils, sync/async model with annotations) with a 6-phase implementation roadmap (M0–M5).

### Improved

- Packaging Option 3 now displays the exact absolute path where the plugin was packaged in the registry cache.
- The Plugin Manager and Developer Tools no longer freeze the UI when opening or running operations. All network calls (registry fetch, GitHub fork/push) and heavy filesystem tasks (git clone, file copy, SHA-256 hashing) run in the background on a Tokio blocking pool and stream progress back to the console.

### Changed

- Pressing Enter on a file now opens it with Pairee's native viewer (text, image, or hex mode) by default, instead of launching an external editor such as `nano`. Files remain editable via F4 and viewable via F3/Alt+F3 as before.
- The previous Enter-to-external behavior (running the association command such as `nano %f`, or `xdg-open`/`start` as fallback) is now opt-in via a new `enter_use_external` toggle in the Editor & Viewer configuration tab.

### Added

- Restructured the plugin registry layout with lowercase single-character partition subdirectories matching the author's initial (e.g., `f/FittyAr/`) to prevent heavy root folder listings.
- A dedicated `plugin-template` git branch (orphan) containing the canonical boilerplate files for new plugins (`manifest.toml`, `main.lua`, `lang/en.toml`, `help/en.md`, `icon.png`, `screenshots/screenshot1.png`). The template is never surfaced in any plugin list in the UI.
- New `clone_from_template()` function in the developer tool that uses the `git2` crate to extract template files directly from the local `plugin-template` branch without requiring an external `git` binary. Placeholder tokens (`PLUGIN_NAME`, `PLUGIN_DESCRIPTION`, `PLUGIN_AUTHOR`) are substituted after extraction.
- `PAIREE_REPO_DIR` environment variable support allowing developers to explicitly point to the Pairee source repository for template resolution.
- Developer Tools Option 0 label now reflects the current state: "Select active development plugin" when none is selected, and "Change / deselect active plugin" when one is active, making the dual deselect/change behavior explicit.

### Fixed

- Active development plugin selection is now validated at application startup; if the previously selected plugin folder no longer exists on disk, the selection is automatically cleared from the configuration file.
- Active development plugin selection is also validated each time Developer Tools are opened at runtime, avoiding stale references after manual folder deletions.
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
- Updated `README.md` and `README.es.md` with a clean, structured design, a modern features list, and a dedicated project status section inspired by an external terminal file manager's layout.
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
