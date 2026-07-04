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

### Improved

- The Plugins Manager runs network and heavy filesystem operations asynchronously in the background, keeping the TUI fully responsive.
- Upgraded the plugin packager to scan and hash files dynamically.
- Improved help documentation by dynamically loading locale help files (`help/<locale>.md`) inside the F1 Help menu.
- Display in the Plugins Manager TUI list now strips the `.pairee` suffix, showing clean, user-friendly plugin names.

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
