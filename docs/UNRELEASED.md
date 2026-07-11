## [Unreleased]

### Added

- High-performance asynchronous Transfer Engine inspired by TeraCopy, allowing non-blocking background file copying and moving.
- Interactive multi-tab Transfer Panel showing file list transfer statuses, options, speed statistics, and logs.
- Support for queueing multiple file transfer jobs with paused, active, skipped, and cancelled lifecycle controls.
- Secure move operation calculating source hashes before moving, verifying destination integrity, and safely removing original files.
- Cryptographic hash verification supporting CRC32, MD5, SHA-1, SHA-256, and BLAKE3 algorithms.
- Automatic HTML and CSV transfer report generation upon completion.
- Local Network (LAN) path auto-detection and buffer size optimization (up to 4MB) to maximize throughput and network fault tolerance.
- Multiplatform Post-Action support (Shutdown, Sleep, Hibernate, and CloseApp) executed automatically upon queue completion.
- Windows Long Paths (Unicode `\\?\`) support in direct I/O operations for filenames exceeding 260 characters.
- Interactive file conflict resolution prompt ("Ask" mode) pausing the transfer pipeline and allowing options for Overwrite, Overwrite Older, Skip, or Rename.
- Interactive Jobs Queue tab inside the Transfer Panel allowing job deletion via the Delete key.
- Persistent folder transfer history (`transfer_history.toml`) saving the last 20 source and destination directories.
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

- Direct I/O implementation now ensures 4096-byte memory alignment using `AlignedBuffer` and handles partial sectors at the end of files by temporarily falling back to standard buffered handles, resolving transfer errors with non-aligned boundaries.
- Symlink replication now correctly creates the link pointer at the destination when `follow_symlinks` is `false`, instead of reading/writing the linked target's content.
- Implemented the `limit_bandwidth_rate` option in the file transfer pipeline, allowing users to configure a speed limit (throttling) to avoid saturating network or local disk buses.
- Implemented `preserve_acl` on Windows NTFS targets to replicate security descriptors (Primary Owner, Group, and DACL access rules) from source to destination.
- Synchronized all `CopyPrompt` interactive dialog selections (write caching, symlink modes, conflict resolutions, and attributes) with the transfer engine worker, preventing interactive choices from being ignored.
- Exposed all advanced transfer options (security descriptors, ADS streams, symlink skipping/following, and bandwidth limit) inside the active Options tab in the TUI, allowing real-time toggle navigation up to index 11.
- Added `transfer_preserve_acl`, `transfer_preserve_streams`, `transfer_follow_symlinks`, and `transfer_limit_bandwidth_rate` parameters to global settings for persistent default configurations.
- Implemented a disk free space pre-check in the transfer worker (using native `GetDiskFreeSpaceExW` on Windows) to report low space warnings before commencing copy operations.
- Implemented the missing `EjectDrive` and `RunScript` post-action executions using Windows PowerShell/Linux commands and tokio subprocess management respectively, with full UI toggle integration.
- Added a `halt_on_error` option (configurable in `Settings` as `transfer_halt_on_error`) to immediately halt and fail the transfer job upon encountering the first file copy error or hash mismatch instead of skipping.
- The Transfer Panel file list now supports stateful navigation, highlighting, and scrolling using the Up/Down arrow keys.
- The Conflict dialog now displays the full source and destination paths of the conflicting files to make resolution more descriptive.
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

- Resolved compiler warnings across the transfer engine, including unused event fields, builders, and trait methods to enforce codebase guidelines.
- Fixed standard Lua bindings registration to use `utils_ext` instead of `utils_basic`, exposing extended scripting utilities (e.g. quote, percent-encode) to Lua plugins.
- Resolved application startup crash (`STATUS_DLL_NOT_FOUND` / `0xC0000135`) on clean Windows installations.
- Fixed UI crash when rendering accented characters in the developer tools console.
- Fixed plugin initialization wizard to properly generate skeleton files when offline by using a built-in fallback template.
- Enforced submission validation checks requiring an `icon.png` and screenshots before packaging a plugin.
- Fixed manifest.toml deserialization to support both flat formats and nested table formats.
- Fixed text wrapping and border overflow issues in the Plugins Manager details panel.
- Fixed terminal stdout corruption when initializing a plugin skeleton.
- Fixed the Search tab in the Plugins Manager not showing any results and not allowing text input on open.
- Fixed Tab key being consumed by the Search text field, preventing navigation to other Plugin Manager tabs while typing.
