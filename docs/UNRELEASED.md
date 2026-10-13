## [Unreleased]

### Added

- Terminal screens of `command &` can be closed (F10, the close box or Delete in the screens list), their process stopped (F8), scrolled back (Shift+arrows, Shift+PageUp/PageDown, mouse wheel) and typed into.
- Setting "Lines kept per terminal screen" (System tab) bounds the output a terminal screen keeps.
- Command line editing: cursor keys, Home/End, Delete, Ctrl+W to delete a word and Tab to complete file and folder names.

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
- `cd` on the command line accepts quoted names (as drag & drop inserts them), `~/folder`, `cd -`, relative paths and SFTP or archive panels, and records the folder history.
- Typed text stays visible on the command line when the prompt is long.
- A double click in a long panel opens the entry that was clicked: the panel scrolls only when the cursor leaves the view, as in Far and Midnight Commander.
