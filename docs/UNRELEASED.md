## [Unreleased]

### Added

### Improved

- Text in the plugin manager, shortcuts list, file associations and undo labels is cut by display width, so wide characters stay aligned.
- Theme colors and highlighting rules are prepared once when they change instead of on every frame.

### Changed

### Deprecated

### Removed

### Fixed

- The plugin manager no longer crashes when a registry plugin has a non-ASCII name or author.
- Plugin manager descriptions no longer freeze the app in a very narrow terminal.
- The panels now use the "Files highlighting" rules from Options instead of always the built-in ones.
