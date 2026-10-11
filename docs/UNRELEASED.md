## [Unreleased]

### Added
- `pairee --version` (`-V`) prints the installed version.
- macOS builds (Intel and Apple silicon) on every release; the built-in updater picks them up.
- Full mouse support where the terminal reports it: panels (click, double click, Ctrl/Shift click, right click or right drag to tag, column titles sort, wheel), drag and drop of files onto the other panel, folders, tabs or the command line, a draggable border between the panels, the menu bar and its drop-downs (a hidden bar shows when pointing at the top row), the function-key bar, and every dialog control (buttons, lists, check boxes, tabs, text fields). Dialogs move by their top border and flowing ones resize by their corner. Plugins get an `on_mouse` event.
- Mouse setting `Auto` / `On` / `Off` (was on/off): `Auto` captures the mouse only where the terminal reports it. `PAIREE_MOUSE=0|1` and `--no-mouse` override it, and the `toggle_mouse_capture` action hands the mouse back to the terminal to select text.

### Improved

### Changed
- Release tooling: `cargo xtask release` replaces the PowerShell and Bash bump scripts and updates every dependency to its latest compatible version before each release. Releases are built only from commits that pass every check, every package is smoke-tested, and a draft is published after approval. A pre-push hook (`cargo xtask install-hooks`) runs rustfmt and the spell check locally.

### Deprecated

### Removed

### Fixed
- With the mouse setting off, Pairee still captured the mouse, so the terminal could not select text.
- Programs started from Pairee (shell commands, file associations, `sudo`) received mouse escape sequences when the pointer moved.
- The `.deb` package declared an invalid dependency (`$default`), so dpkg and apt refused to install it.
- The Windows installer no longer asks for administrator rights: it installs for the current user, as it always did, and new terminals see the updated PATH right away.
