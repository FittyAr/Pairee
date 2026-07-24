## [Unreleased]

### Added

- Interactive dialog for file associations enabling navigation, addition, editing, and deletion, with clear visual prompts and helper hints on keys to use.
- Expanded Git support with comprehensive backend APIs for individual file staging, unified diffs, remote syncing (fetch, pull, push), advanced branch management, stashing, resets, merges, and repository clone/initialization.
- New Git dashboard TUI integration with an interactive 4-tab panel (Status, Log, Branches, and Stash).
- Unified diff viewer modal with syntax-colored lines for additions, deletions, and hunks.
- Interactive popup dialogs for stash creation, branch creation/renaming, and safe confirm-action dialogs for resets, merges, and stashes.
- New Spanish translation and updated English manual for Git integration reference.
- New `F7` Rename action that prompts only for the new filename (with a live collision warning if a sibling already exists).
- `Rename` command added to the **Top Menu Bar → Files** submenu.
- F-key shortcut bar now reads each slot from the active keybinding resolver, so the bar always shows what each F-key actually does.
- `Create folder` (MkDir) action added as a default option in the **User Menu** (`F2`), bindable to key `6`. The action opens the same name prompt dialog used everywhere else.

### Improved

- Alt+G Git panel initialization now populates stash data immediately on launch.
- F2-F12 F-key shortcut bar now matches the actual action each key triggers: F2 = User Menu, F9 = Top Menu, F7 = Rename, F11 = empty (when not bound).
- Bottom F-key bar no longer claims `F11 = Plugin` by default — the F11 slot now renders blank until the user explicitly rebinds the key.

### Changed

- Expanded default file association presets to support a wide range of popular formats (text, code, images, audio, video, documents, and web pages).
- F6 dialog renamed from "Rename/Move" to "Move" only — Rename is its own modal now.
- `Make Folder` and `Plugin commands` moved out of the F-key bar into the **Top Menu Bar → Files** submenu so the bar can focus on the most frequent operations.
- The plugin system (`PluginMenu` action) is no longer reachable from `F11`. It is now accessible exclusively via **Top Menu Bar (`F9`) → Files → Plugin commands**. Power users can still rebind `F11` to `plugin_menu` in `keybindings.toml` if they prefer the old layout.

### Removed

- Default keymap no longer binds `F7` to `MkDir` or `F11` to `PluginMenu`. `MkDir` lives in the User Menu (F2) and `PluginMenu` lives under the top menu bar (F9 → Files). Power users can still rebind the keys in `keybindings.toml`.

### Fixed

- Single file copy target path resolution so copying a file to a target destination path no longer creates an extra directory with the file name.
- F-key bar in `keymaps/*.toml` preset files now reflects the new keymap (F7→Rename, no F7→MkDir, no F11→PluginMenu), so users upgrading keep the bar and behavior in sync.
- Outdated doc comment on `Action::MkDir` that still claimed the action was bound to `F7`.
