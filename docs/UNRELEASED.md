## [Unreleased]

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
