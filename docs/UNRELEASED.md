## [Unreleased]

### Added

- Support for installing multiple plugins in a single `pairee plugin install` or `pairee plugin add` command, dynamically normalizing names to support both raw names and `.pairee` extensions.
- New `enter_use_external` setting (default `false`) to manually enable launching external association commands (e.g. `nano %f`) when opening files with Enter. Exposed in the Editor & Viewer settings tab.
- Remote blocklist support to disable or hide unsafe and broken plugins from search and remote listings.
- Asynchronous loading in the Plugins Manager (F11) with a status spinner while the registry index is fetched.
- A new interactive TUI Plugins Manager (F11) featuring a tabbed interface to manage installed plugins, toggle trust/pinned statuses, search the remote registry, and perform background installations.
- A TUI Developer Tools tab under the Plugins Manager (when `plugins_developer_mode` is enabled) featuring a console to initialize plugin skeletons, run lint audits, package plugins, and format submissions.
- Step-by-step interactive assistant wizard (in both TUI and CLI) to initialize new plugin skeletons.
- Active development plugin selection system in the Developer Tools tab, allowing developer operations to target a specific development plugin.
- CLI commands `check-updates` and `update` in the `pairee plugin` suite to query and apply plugin updates.
- Interactive modal selection list for Developer Tools Option 0 (Select Active Plugin), displaying all detected plugins from `plugins_dev_dir` as well as any plugin found in the active Panel 1 or Panel 2 directories, navigable with keyboard arrows and confirmed with Enter.
- Auto-selection of newly created plugins in Option 0 upon successful initialization in Option 1.
- Automatic license detection and auto-assignment during packaging, prompting for license names when present but undeclared, or auto-assigning `"MIT"` and generating a standard `LICENSE` file if not present.
- Live progress bar and status text in the Developer Tools console that stream coarse-grained milestones (e.g. "Cloning registry…", "Copying file 3/12…", "Computing SHA-256…") during long-running operations.
- Three new navigation items in the Developer Tools menu (6, 7, 8) to move the active file panel directly to the dev plugin folder, the packaged plugin folder, and the submit folder.
- Plugin system evolution — Phase 0 (Scaffolding) implemented: unified `pairee.emit(action, args)` action dispatch, structured `pairee.notify({title, content, level, timeout})` API, `pairee.file_cache({file, skip})` cache path helper, `pairee.which({cands, silent})` key-prompt API, structured `pairee.confirm({pos, title, body})` and `pairee.input({pos, title, value, obscure, realtime, debounce})` variants (TUI popups to ship in M1), `pairee.utils.*` namespace exposing `target_os()`, `target_family()`, `time()`, and `hash(str)`, and deprecation warnings on legacy `pairee.app.confirm` and `pairee.app.input` stubs; 27 new unit tests added (81 total).
- New user-facing plugin reference: `help/en/plugins.md` and `help/es/plugins.md` — complete documentation of the plugin Lua API surface including M0 entries, with a migration cheatsheet for legacy `pairee.app.*` stubs.
- Technical documentation: `docs/technical/plugin-roadmap.md` and `docs/technical/plugin-roadmap-es.md` — internal design document enumerating 14 gaps in the current plugin system with a 6-phase implementation roadmap (M0–M5).
- SRP refactor of plugin manager: `src/plugin/manager.rs` (750 lines) split into 6 focused files under `src/plugin/manager/` — `mod.rs`, `snapshot.rs`, `request.rs`, `manager.rs`, `dispatcher.rs`, `dispatch_actions.rs`. Public API preserved unchanged.
- SRP refactor of dev tools: `src/app/input_popup/plugin_menu/dev.rs` (924 lines) split into 5 focused files under `src/app/input_popup/plugin_menu/dev/` — `mod.rs`, `paths.rs`, `progress.rs`, `options.rs`, `select_popup.rs`.

### Improved

- The Plugins Manager runs network and heavy filesystem operations asynchronously in the background, keeping the TUI fully responsive.
- Upgraded the plugin packager to scan and hash files dynamically.
- Improved help documentation by dynamically loading locale help files (`help/<locale>.md`) inside the F1 Help menu.
- Display in the Plugins Manager TUI list now strips the `.pairee` suffix, showing clean, user-friendly plugin names.
- The Search tab in the Plugins Manager now pre-loads all available plugins from the registry on open and filters results in real time as the user types, without requiring a separate query submission.
- Plugin search results are now displayed in a tabular layout (Name · Author · Version) matching the file panel style, with `.pairee` suffix hidden.
- Arrow keys Up/Down navigate the plugin list in the Search tab even while typing a query. PgUp/PgDn paginate through long lists.

### Changed

- Pressing Enter on a file now opens it with Pairee's native viewer (text, image, or hex) by default. Launching external editors on Enter is now opt-in via the `enter_use_external` config setting.
- Replaced the hybrid translation engine with a portable, symmetric TOML-based system using embedded files (`lang/en.toml` and `lang/es.toml`), supporting custom local overrides.

### Fixed

- Resolved application startup crash (`STATUS_DLL_NOT_FOUND` / `0xC0000135`) on clean Windows installations.
- Fixed UI crash when rendering accented characters in the developer tools console.
- Fixed plugin initialization wizard to properly generate skeleton files when offline by using a built-in fallback template.
- Enforced submission validation checks requiring an `icon.png` and screenshots before packaging a plugin.
- Fixed manifest.toml deserialization to support both flat formats and nested table formats.
- Fixed text wrapping and border overflow issues in the Plugins Manager details panel.
- Fixed terminal stdout corruption when initializing a plugin skeleton.
- Fixed the Search tab in the Plugins Manager not showing any results and not allowing text input on open.
- Fixed Tab key being consumed by the Search text field, preventing navigation to other Plugin Manager tabs while typing.
