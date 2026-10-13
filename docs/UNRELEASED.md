## [Unreleased]

### Added

- Terminal screens of `command &` can be closed (F10, the close box or Delete in the screens list), their process stopped (F8), scrolled back (Shift+arrows, Shift+PageUp/PageDown, mouse wheel) and typed into.
- Setting "Lines kept per terminal screen" (System tab) bounds the output a terminal screen keeps.

### Improved

- Text in the plugin manager, shortcuts list, file associations and undo labels is cut by display width, so wide characters stay aligned.
- Theme colors and highlighting rules are prepared once when they change instead of on every frame.

### Changed

- The screens list and the command-line screen indicator are translated.

### Deprecated

### Removed

### Fixed

- The plugin manager no longer crashes when a registry plugin has a non-ASCII name or author.
- Plugin manager descriptions no longer freeze the app in a very narrow terminal.
- The panels now use the "Files highlighting" rules from Options instead of always the built-in ones.
- Output of a background command no longer goes to the wrong screen, or gets lost, after an earlier screen is closed.
