# Changelog

All notable changes to Pairee will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/):

- `Added` for new features.
- `Changed` for changes in existing functionality.
- `Deprecated` for soon-to-be removed features.
- `Removed` for now removed features.
- `Fixed` for any bug fixes.
- `Improved` for performance or UX improvements.

---

## [v0.9.0] - 2026-10-09

### Keyboard

- **Presets for the program you come from.** Four keymaps keep the keys of their programs: *Norton Commander / Far* (the default), *Standard* (Windows Explorer, VS Code, Total Commander), *Neovim* (Vim, vifm, oil.nvim, nvim-tree, with a `Space` leader) and *yazi*. Each says what plain letters do: type into the command line (Far), jump to a file (Explorer) or act as commands (Vim, yazi). Key sequences (`g g`, `Ctrl+w h`), the leader key and Norton's `Alt`+letter quick search are supported. The F1 help has a reference page per preset (generated from the presets, `pairee keymap print --preset <name> --lang <code>` prints it) and a "Coming from another program" guide.
- **Keyboard shortcuts list** (`Ctrl+K` in Norton, `Ctrl+K Ctrl+S` in Standard, `g?` in Neovim, `~` in yazi, **Options → Keyboard shortcuts...** in the `F9` menu, or **Options → Configuration → Interface**): every command of the panels, editor, viewer and dialog lists with its keys, searchable by name or by pressing a key. `F2` rebinds, `Ins` adds a key, `Del` removes, `F8` restores, `Ctrl+←/→` previews another preset, `F9` saves your preset. Changes apply at once. It replaces the which-key overlay; the command palette now shows each action's key.
- **New actions** available in every preset: file clipboard (yank / cut / paste, paste overwriting, paste as link), recycle bin vs. permanent delete, select all, visual (range) selection, back / forward through visited folders, home and root folder, half-page moves, search in the panel with next / previous match, create a file or folder, rename the name only, copy name / folder path, focus a given panel, cycle view and sort, and Far's command-line keys (`Ctrl+Enter`, `Ctrl+F`, `Ctrl+E`/`Ctrl+X`, `Ctrl+Y`) and `Ctrl+P` / `Ctrl+B`.
- **Editor, viewer and dialog lists** take their keys from the preset too, and help, screens, palette and the shortcuts list work over the editor and viewer with whatever key they have.
- **Plugins** declare commands with suggested keys per preset (`[[commands]]` in `manifest.toml`); they join the keymap, the palette and the shortcuts list, never take a key the preset uses, and can be rebound. `pairee.keymap.list()` / `chord_for()` read the keymap and `pairee.emit()` runs any action.

### Changed (keyboard)

- *Norton Commander / Far* follows NC/Far again: `F7` makes a folder, `Shift+F6` renames, `Ctrl+Shift+F6` is the multi-rename tool, `F11` opens plugins, `Ctrl+\` goes to the root, and `Alt`+letter is the quick search. Pairee's own keys moved to `Ctrl+Alt`+letter: tabs `Ctrl+Alt+T`/`W`/`O`, Git `Ctrl+Alt+G`, SSH `Ctrl+Alt+R`, folder sizes `Ctrl+Alt+S`, disk usage `Ctrl+Alt+D`, hotlist `Ctrl+Alt+B`, quick filter `Ctrl+Alt+F`; redo is `Alt+Shift+Backspace`.
- The *VSCode* preset is now *Standard* (`vscode` still works as a name).
- `Alt+F3` opens the alternative viewer again.
- The `F2` user menu offers new tab (`T`), open in a new tab (`O`) and close tab (`W`) while `usermenu.toml` defines no commands of its own.

### Migration

- Your keymap is updated automatically when the files in `keymaps/` are copies Pairee wrote; a copy you edited is kept and the new version is saved next to it as `<name>.toml.new`.
- `[custom_bindings]` in `keybindings.toml` becomes `[overrides.all]`. An override now replaces the action's keys and takes the key from the action that had it, instead of being rejected.
- Plugin `[keybindings]` tables keep working; `[[commands]]` is the new format.

### Added

- Undo and redo of file operations: `Alt+Backspace` / **Files → Undo** reverses the last rename, multi-rename, move, copy (only the new copies are deleted, never a file that overwrote another one), make folder, create link or send to trash (restored from the Recycle Bin / trash on Windows and Linux), and `Ctrl+Y` / **Files → Redo** repeats it. The menu shows the operation (*Undo: Move (3 items)*); a confirmation lists what will be reversed. Every entry is checked first (still there with the same size and date, original location free) and whatever changed is skipped and reported, never overwritten. Undo and redo run through the Transfer Engine and the rename executor. The journal keeps the last 50 operations in memory; permanent delete, wipe and SSH transfers are recorded as not undoable.
- Multi-rename tool (`Ctrl+Shift+F6` in Norton, **Files → Multi-Rename**) for the selected items: name and extension masks with `[N]`, `[N2-5]`, `[E]`, `[P]`, `[C]` (start, step, digits) and `[Y][M][D][h][m][s]` placeholders, search & replace (plain or regular expression, optional case-insensitive), case transform, and a live old → new preview that flags empty, invalid, duplicate and already-existing names. Renames are ordered so nothing is overwritten (swaps go through a temporary name) and are undone if one fails. Works on local and SSH panels.
- **Commands → Synchronize folders** (Total Commander "Synchronize dirs"): compares both panel folders recursively in the background (size and modification time with the configurable tolerance, optionally file contents with the transfer hash algorithm; filter mask with include/exclude globs; hidden files can be ignored), lists every difference with an action per item (copy →, copy ←, delete, skip) and totals of files and bytes, and applies the plan as Transfer Engine copy/delete jobs. Directions: left → right, right → left, both ways (newer wins) and mirror (deletes extras on the right, after an explicit confirmation).
- Folder shortcuts can now be assigned: **Commands → Folder shortcuts** lists slots `Ctrl+Alt+1` … `Ctrl+Alt+9`; `Ins`/`Space` (or the slot digit) assigns the current folder and `Del` clears it. Shortcuts are saved in `bookmarks.toml` in the config folder.
- The directory hotlist (`Ctrl+\`, also **Commands → Directory hotlist**) is now persistent: `Ins`/`+` adds the current folder, `Del`/`-` removes an entry. Default entries are localized.
- The Git panel has a key in every built-in keymap, as the menu already advertised.
- Built-in keymaps now pick up newly shipped bindings even when an older copy of the keymap file exists in the config folder (user-defined chords always win).
- Editor and viewer honor the "Tab size" settings when displaying tab characters.
- Built-in editor: undo/redo (`Ctrl+Z` / `Ctrl+Y`, typing runs are one step), save as (`Shift+F2`), `Tab` key, `Home`/`End` and `Ctrl+Home`/`Ctrl+End`, horizontal scrolling for long lines, and `F6` in the viewer opens the file in the editor. Line endings (LF/CRLF), the final newline and the UTF-8 BOM are preserved on save. Saving asks before overwriting a file that another program changed or an existing "save as" target. Files that are not valid UTF-8 or larger than 64 MiB are refused with an explanation instead of being corrupted.
- Built-in editor: text selection with `Shift` + arrows/`Home`/`End`/`PgUp`/`PgDn`/`Ctrl+Home`/`Ctrl+End`, `Ctrl+A` and mouse drag (the mouse wheel scrolls); vertical block selection with `Alt+Shift` + arrows or `Alt` + drag; copy/cut/paste with `Ctrl+C`/`Ctrl+X`/`Ctrl+V` (also `Ctrl+Insert`/`Shift+Delete`/`Shift+Insert`) through the system clipboard, falling back to an internal clipboard over SSH or without a display. Typing, `Backspace` and `Delete` replace the selection, each paste or cut is one undo step, and bracketed paste inserts every pasted line. The key bar shows `Shift+F2` (save as) and `Shift+F7` while `Shift` is held, and `F3` (next match).
- Editor settings that had no effect now work and are back in **Options → Configuration → Editor/Viewer**: expand tabs, auto indent, show line numbers, cursor at the end, lock editing of read-only files and warn when opening read-only files.
- Panel visibility, view mode, sort order and long-names mode from the last **Save setup** are the defaults of the panels when there is no session to restore.
- Session restore (new setting **Restore last session**, `restore_session`, on by default, **Options → Configuration → Interface**): on exit Pairee saves every tab of both panels (folder, archive, view, sort, filter, name, lock, entry under the cursor), the shown tab per side, the focused side, panel visibility and Quick View to `session.toml` (atomic write, owner-only on Unix) and reopens them at startup. Missing folders open their nearest existing parent; SFTP tabs are saved by SSH preset name (never a password) and reconnect only when shown, falling back to the home folder with a notice; a damaged file is ignored with a logged warning and kept as `session.toml.bak`.
- Command-line folders: `pairee <left> [<right>]` opens them in the shown tab of each panel (overriding the restored ones; a file opens its folder with the cursor on it). `--cwd-file <file>` writes the focused panel's folder on exit so a shell wrapper can `cd` there, and `--print-cwd` prints it; wrapper functions for bash, zsh, fish and PowerShell are in the user guide and README.
- The "natural" sorting collation now sorts numbers inside names numerically.
- `pairee.fs.data_dir()` returns the plugin's private data directory, the only location an untrusted plugin may write to.
- Support for untracked files in the Git diff viewer, allowing inspection of newly added files with full contents.
- Home and End key navigation in the unified Git diff viewer.
- Context-sensitive empty list notifications across all Git panel tabs (Log, Branches, Stash, and Tags).
- Full suite of unit tests for file diffs (staged, unstaged, untracked), commit diffs, and stash diffs.
- Clipboard paste support (`Ctrl+V`) in the Git clone dialog to easily paste repository URLs and target directory names.
- Detection and notice for modified and untracked binary files in the unified Git diff viewer.
- Guard and modal error notification preventing deletion of the currently checked-out branch in the Git panel.
- Validation guard preventing commit amend (`Ctrl+A`) on repositories without prior commits.
- Protection and modal alert preventing push and pull operations while HEAD is in detached state.
- Mode localization (`Soft`, `Mixed`, `Hard`) in Git commit reset confirmation prompts.
- Informative notice when attempting to commit with clean working copy and no amend mode.
- Internal viewer (F3) and Quick View detect the text encoding: byte-order mark (UTF-8, UTF-16 LE/BE), UTF-16 without a mark, UTF-8, and otherwise the most likely legacy code page via `chardetng` (Windows-1252/Latin-1, other Windows/ISO code pages, Shift-JIS, EUC, GBK, Big5...), decoded with `encoding_rs`. Text in another encoding is no longer treated as binary. The viewer shows the encoding in its new status line and `F8` opens a list to switch it manually. New settings `viewer_autodetect_codepage` and `viewer_default_codepage` (**Options → Configuration → Editor/Viewer**).
- Plugins can declare the programs they need in `manifest.toml` (`[permissions] commands = [...]`); the Plugin Manager shows them in the plugin details.
- Folder sizes: `Space` or `F3` on a folder, or **Commands → Folder sizes** (selected folders, or all of them), computes the size in the background and shows it in the size column and status line until the directory changes; `Esc` cancels. Symbolic links are not followed, hard links count once on Unix, and folders that could not be read completely are marked with `+`. Works on SFTP panels too.
- Disk usage view (**Commands → Disk usage**): ncdu-like list of the current folder sorted by size with percentages and bars, cancellable scan with live progress, navigation into subfolders and back, deletion through the regular delete confirmation and rescan (`r`/`F5`). Scan results are cached per folder.
- Archives as folders: `Enter` or `Ctrl+PgDn` (new `open_archive` action) on a zip, tar, tar.gz/tgz or 7z file opens it in the panel; subfolders open with `Enter`, the title shows the path inside the archive and `..` (or `Backspace` / `Ctrl+PgUp`) at its root returns to the containing folder with the cursor on the archive. `F5` copies the selection out through the Transfer Engine with the safe extractor (no traversal, no writing through links, no overwrite, size limits). Zip archives also accept copies into them, new folders (`F7`) and deletions (`F8`), applied by rewriting the archive to a temporary file and atomically replacing it; tar and 7z archives are read-only. Unsupported actions inside an archive (and archives on SSH panels or inside other archives) show a clear message. Plain `.tar` files are now recognized, and `F7` on an SFTP panel now creates the folder on the server.

- Folder tabs per panel (Double Commander / Far style): a new tab (`Ctrl+Alt+T` in Norton, `Ctrl+T` in Standard) opens on the current folder, `Ctrl+Alt+O` or `Ctrl+Enter` opens the folder or archive under the cursor in a new tab, `Ctrl+Alt+W` closes a tab (never the last one), `Alt+PgDn`/`Alt+PgUp` (also `Alt+Right`/`Alt+Left` in Norton) switch, `Alt+Shift+PgUp`/`Alt+Shift+PgDn` move a tab and `Alt+1` … `Alt+9` jump to a tab. Each tab keeps its own folder and source (local, SFTP, archive), cursor, selection, view, sort, filter and folder sizes, and keeps loading in the background while another tab is shown. A one-line tab bar appears above a panel with more than one tab (new setting **Always show tab bar**, `always_show_tab_bar`); titles are shortened with `…`, a click switches tabs and a middle click closes one. **Left/Right → Tabs** lists every tab command, plus **Rename tab** and **Lock tab** (a locked tab stays on its folder and opens other folders in new tabs). `Ctrl+Tab` and `Ctrl+W` keep switching screens and opening the task list.
- Automatic panel refresh: the folder shown by each side's active tab is watched with native notifications (`notify`, non-recursive) and reread in the background when another program changes it, keeping cursor and selection and without the "Loading…" title; Git badges are updated with the listing and computed folder sizes of changed folders are measured again. Changes are coalesced (250 ms of quiet, at most 2 s during a sustained burst such as a large copy: about 6,000 events in 3 s give 2 rereads). Network shares (SMB/NFS mapped drives, UNC and `\\wsl$` paths), 9p/drvfs/FUSE mounts on Linux and folders whose watch cannot be set up are polled instead (modification time + entry count). Folders above `disable_panel_update_object_count` are not refreshed automatically at all, as the setting's name says. Archive tabs are not watched; SFTP tabs are polled over their connection (`stat` + listing through the panel source) when the new setting **Also poll SFTP panel folders** (`auto_refresh_ssh`, off) is on, every **SFTP poll interval** (`auto_refresh_ssh_poll_secs`, 30 s). New settings **Refresh panels automatically when folders change** (`auto_refresh`, on) and **Poll interval for network folders** (`auto_refresh_poll_secs`, 3 s) in **Options → Configuration → Panel**.
- SFTP panels get back what the panel source refactor had marked as unavailable: renaming in place works on the server (refusing an existing name, undoable), `F4` edits a remote file through a private local copy that is uploaded on save after checking that the original did not change meanwhile (size and time; asks before overwriting), and `Ctrl+A` shows permissions, numeric owner and modification time and changes the mode with SFTP `setstat` (`chmod`). The same `F4` flow edits files inside zip archives. A move between two panels on the same SFTP connection (a server-side rename) is now journaled and can be undone; other SSH copies, moves and deletions stay "not undoable".
- `.tar.bz2` / `.tbz2` / `.tbz` and `.tar.xz` / `.txz` archives open as folders, list in Quick View and extract natively (pure-Rust `bzip2` and `lzma-rust2` decoders, so static musl builds are unaffected). A bare `.bz2` / `.xz` file is not treated as a folder.
- Archives inside archives open as folders (read-only): the inner archive is extracted to a private temporary file, in the background on its first listing and up to 512 MiB, then listed, viewed and previewed like any other archive; `..` at its root returns to the containing archive. Copies, edits and deletions inside a nested archive are refused with a message.
- Default keys for **Folder sizes** and **Disk usage** in every built-in keymap (`Ctrl+Alt+S` / `Ctrl+Alt+D` in Norton; a test checks it).

### Improved

- **Commands → Compare folders** is now recursive (a folder on both sides is reported as different when anything inside differs) and runs in the background with a progress popup; `Esc` cancels it.
- The Screens menu and the development-plugin picker now use the theme's selection colors, like the other list popups.
- Text fields in dialogs (make folder, rename, copy/move destination, compress, apply command, describe, link, filters, select group) now share one input box: the cursor can be moved with Left/Right/Home/End, Delete removes the character under it, `Ctrl+V` pastes from the clipboard, and non-ASCII text is edited by whole characters.
- Panels are now read in the background: the directory listing, SFTP listing, Git status and free space no longer block the interface, the panel title shows "⟳ Loading…" while a read is in progress, and the cursor and selection are kept when the new listing arrives. Entering a directory rereads only that panel, and Git status is computed only for the listed directory instead of the whole repository.
- Git fetch, pull, push, push tags, remote branch deletion and clone now run in the background with a progress popup (objects and bytes transferred); `Esc` cancels the operation. The Left/Right menus no longer open the repository while drawing.
- Quick view loads previews in the background after the cursor settles (no more stalls when scrolling through large files), caches recent previews per file version, and reads at most 16 MiB per file. The F3 viewer reads files page by page through a small block cache with a line index built in the background, so multi-gigabyte files open instantly with bounded memory (the status line shows the line count growing while indexing); hex mode uses the same paged reads. Viewer search (`F7`, `F3` to repeat) runs in the background over the whole file, shows its progress and is cancelled with `Esc`. Quick View reads only the first 256 KiB of a text file. The viewer shows the error instead of an empty viewer when the file cannot be read.
- The Git panel opens immediately and reads status, log, branches, stashes and tags in the background (the title shows "⟳ Loading…"); refreshing after an operation or with `r`/`F5` keeps the current contents on screen until the new data arrives, outdated results are discarded, and older log pages load in the background while scrolling. Loading the panel no longer reads the repository on the UI thread. After applying a stash the success notice now opens over the panel instead of closing it.
- Local Git panel operations (stage, unstage, stage/unstage all, discard, add to `.gitignore`, file/commit/stash diffs, commit, stash save/apply/pop/drop/clear, branch create/rename/delete/checkout, merge, rebase, reset, cherry-pick, revert, tag create/delete, remotes) no longer touch the repository on the key handler: they run as background jobs (one generic `app::git_local` job), the panel title shows "⟳ Working…" while one runs, the panel is then re-read by the background loader, and failures that used to be silent (stage, unstage, `.gitignore`, diff) now open the localized error over the panel. Success and conflict notices open over the panel instead of closing it.
- Quick view loads previews in the background after the cursor settles (no more stalls when scrolling through large files), caches recent previews per file version, and reads at most 16 MiB per file. The F3 viewer also loads in the background, reads at most 64 MiB (larger files are shown truncated with a notice) and shows the error instead of an empty viewer when the file cannot be read.
- Detailed, Owners and Links views no longer read file attributes on every repaint, and the free-space footer no longer queries the disk on every repaint; both are refreshed with the listing.
- Release builds now use thin LTO, a single codegen unit and stripped symbols for a smaller, faster binary.
- Untracked file badges and labels in Git panel now render in Magenta for consistent contrast and readability across dark backgrounds.
- Enhanced scroll behavior in the Git diff viewer for short files.
- Centralized all Git operation error alerts, conflict notifications, confirmation prompts, and buttons into localization catalogs with zero hardcoding.
- Internal: the "do not overwrite an invalid `config.toml` until confirmed" lock and the startup load error now live in `AppConfig` (`ConfigLoadState`) instead of process-wide globals.
- Internal: the panel source port gained an `attributes` capability (`Vfs::attributes` / `Vfs::set_attributes`), and one-off background changes of a non-local source (`AppState::start_vfs_task`) return a follow-up command that runs on the UI thread (open the editor, show a dialog).
- Internal: panels, folder sizes, the disk usage view, multi-rename and SSH copy/move/delete now go through one panel source port (`fs::vfs::Vfs`, Ports & Adapters) with local and SFTP adapters and capability flags, replacing the separate `DuSource`, `RenameBackend` and `RemoteFs` traits and the per-feature "local or SSH" branches. Panels keep a `PanelSource` instead of an optional SSH connection. Folder links inside a tree uploaded or downloaded over SSH are created as folders but not followed.
- The viewer (`F3`) and Quick View read through the panel's source: entries of an archive panel and files on SFTP panels open in the viewer and preview (read into memory, up to 64 MiB; local files are still paged from disk). Images are decoded from the bytes read, so they preview inside archives too.
- Internal: each panel side now holds a list of folder tabs (`app::state::tabs`); a tab owns the whole panel state (location and source, listing, cursor, selection, view, sort, filters, folder sizes) and its background jobs, identified by a stable `TabId`, so listings and SSH connections finish in the tab that started them even after switching tabs. Tabs describe themselves as a plain, serializable `TabSpec`.
- Internal: `bookmarks.toml`, `history.toml` and the new `session.toml` share one TOML state-file store (`config::toml_store`): tolerant reads and atomic writes (owner-only `0600` on Unix); the history file is no longer rewritten in place.
- **Commands → Compare folders** reads both panels through their source, so a folder inside an archive or on an SFTP server can be compared with any other panel (content hashes are computed from the source too). **Synchronize folders** also works with folders inside archives (zip archives accept the copies and deletions); only SFTP panels are refused, with a clearer message.
- Internal: no function is longer than 100 lines any more (`clippy::too_many_lines` is enforced in CI): long key, action and plugin-request dispatchers became per-group handlers and tables, and long renderers small render steps; duplicated code is down to 0.73% (jscpd).

### Deprecated

### Removed

- External editor support: Pairee edits files only with its built-in editor. F4 and every other "edit" entry point always open the internal editor. The `editor_use_external` and `default_editor` settings are gone (old `config.toml` files that still contain them load fine), and the default file associations no longer launch `notepad`/`nano` for text files; untouched old default rules of that kind are removed from `associations.toml` on load.
- Settings that had no effect (file descriptions, info panel details, dialog/command-line editing, editor code pages and blocks, most viewer options, plugin manager flags, `git_auto_detect`, `transfer_engine_enabled` and some confirmations) were removed from the configuration, the dialog and the manuals. Old `config.toml` files that still contain them load fine; the keys are dropped on the next save. See `docs/technical/settings-audit.md`.
### Fixed

- Keymaps: the `Ctrl+Shift+<letter>` bindings of the shipped presets (copy path, command palette, key overlay, SSH and, in VSCode, make folder, view, history, refresh, tree, quick view) were rejected by the key parser and never worked; they are now written as `Ctrl+<UPPERCASE>` and the old spelling is accepted in user files. The VSCode keymap no longer binds `F2` twice (user menu moved to `Alt+U`) nor `Ctrl+F` twice (find file is `Ctrl+Shift+F`), swap panels is `Ctrl+U` (it collided with SSH), `Ctrl+,` (settings, written `Ctrl+Comma`) and `Ctrl+.` (hidden files) now load, and the NeoVim keymap lost an unknown `change_panel_tab` entry. A test loads the three presets and requires zero errors, warnings and duplicate chords.
- F-key bar: the panel rows (plain, `Shift`, `Ctrl`, `Alt`) are now built from the active keymap through one action → label table, so `Shift+F1`–`F3` (pack, unpack, archive commands), `Shift+F9` (save setup), `Shift+F10` (context menu), `Alt+F11`/`Alt+F12` (histories) show their labels, and keys a keymap leaves unbound are blank instead of showing Norton defaults. `Ctrl+P` now also cycles to the `Shift` row.
- Built-in editor: block selection without `Alt+Shift` + arrows, which Windows Terminal keeps for itself: `Ctrl+B` toggles block mode (the status line shows it), where `Shift` + arrows and mouse drags select a vertical block; `Ctrl+Alt+Shift` + arrows also select a block.
- Multi-rename preview: the table has a scrollbar (drag or click) and scrolls with the mouse wheel. The mouse wheel now scrolls the list or text under the pointer (Git panel, histories, help, transfer lists, panels…) instead of always the topmost one, three rows per step; away from any scrollable view it keeps scrolling the topmost.
- Disk usage view: deleting an item on an SFTP panel left it in the list until a rescan; deleted items now leave the tree when the delete job finishes, for every source, instead of the view polling the local disk on every frame.
- Folder sizes were kept after a reread of the same folder even when a folder's contents had changed; folders whose modification time changed in the new listing are now measured again (watcher-reported changes still invalidate at once).
- Copying into a zip archive no longer silently replaces entries that already exist: the copy dialog's conflict choice (overwrite, overwrite older, skip, rename, ask with the usual conflict dialog) applies, through the same resolver as local copies. Copies into or out of archives and deletions inside them now appear in the undo journal as not undoable instead of not at all.
- Auto-refresh: a file created or deleted right after entering a folder (while its watch was still being set up) never showed up until the next change; a folder modified just before its monitor is armed is now reread once.
- Archives: copying a single item out of an archive (or a single file into a zip) with the destination `F5` suggests (`<folder>/<name>`) created a folder with the item's name and put the item inside it (`b.txt/b.txt`); the suggested path is now the target itself, as for local copies.
- Copy/move filter masks: an exclusion such as `!target` now also skips folders with that name (it only applied to files, so excluded folders were still copied in full).
- Folder compare: files copied to or from FAT/exFAT drives were reported as "Different" because the modification-time tolerance was 1 second; it is now 2 seconds and configurable with `compare_mtime_tolerance_secs` in `config.toml`. On Windows and macOS names are matched ignoring case (`README.txt` and `readme.txt` are the same file there), and comparing a remote (SSH) panel now shows a clear message instead of reading a local path with the same name.
- Configuration dialog: editing the plugin developer folder now shows the text being typed (it kept showing the old path), and text fields there support cursor movement and paste like other dialogs.
- Search highlighting in the viewer and editor no longer misaligns (or can panic) on lines containing characters whose lowercase form is longer, such as 'İ'.
- Git panel: the "rename branch" prompt opened with no field focused (typing did nothing until the arrows were pressed); it now starts on the name field.
- Create link: typing `s` or `h` in the link name switched between symbolic and hard link instead of inserting the letter; the link type is now toggled with `Tab` (shown in the dialog hint).
- Move (F6) with "Confirm move" turned off ignored the transfer settings (it always disabled attribute preservation, write-cache bypass and symlink handling and asked on conflicts); it now uses them exactly like Copy does.
- Removed several crash paths: Git panel sub-dialogs, copy/move filter and tree prompts no longer `unwrap` the open dialog, file panels never slice out of range while drawing, and popups are no longer cloned on every key press (the Git panel and image quick view were deep-copied per key).
- Transfers: a panic in one transfer no longer cascades into crashes of the whole application through poisoned locks; the "file exists" prompt no longer polls every 100 ms (the worker is woken by the answer or by cancelling the job), and copy progress updates are coalesced so a fast copy cannot flood the interface.
- File owner names on Linux/macOS: `/etc/passwd` is parsed once per session instead of being re-read for every file whose owner is unknown.
- SSH: blocking SSH/SFTP calls now time out after `ssh_timeout_secs` seconds (new setting in `config.toml`, default 30; 0 disables it) instead of hanging forever on a dead server. Connecting and SSH copy/move/delete jobs run on the blocking thread pool, and the panel title no longer locks the SSH session while drawing.
- SSH: deleting a remote folder with nested subfolders failed because a directory was removed before its subdirectories were emptied; remote recursive delete now removes children first, and symbolic links to directories are removed as links instead of being followed.
- Compressing a folder to ZIP now keeps its subfolder structure; previously file names were glued together (e.g. `projectsubfile.txt` instead of `project/sub/file.txt`).
- The "Save commands / folders / view and edit history" settings are now honored: disabled categories are no longer written to or restored from `history.toml`.
- Symbolic links to directories are listed and opened as directories instead of files.
- On Windows, files with the Hidden attribute are hidden unless "Show hidden files" is on.
- Delete to Recycle Bin now uses the system trash on Linux, macOS and Windows; when the trash is unavailable the item is kept and an error is shown, instead of being deleted permanently.
- "Disable panel update if object count exceeds" no longer prevents opening a large directory; it only skips automatic rereads of the same directory, and `Ctrl+R` always rereads.
- Keyboard shortcuts help: removed the duplicated `Ctrl+L` entry, repaired the selection table, documented that `Ctrl+H` / `Ctrl+I` / `Ctrl+M` need the kitty keyboard protocol (with rebinding examples), and fixed broken links to other manuals.
- Fixed crashes in the built-in editor, the Git commit prompt and editor search when typing or moving the cursor over non-ASCII text (e.g. `ñ`, `á`, emoji, combining accents); cursor movement, Backspace and Delete now operate on whole grapheme clusters via a shared text-input helper.
- Fixed the terminal being left in raw mode / alternate screen with mouse capture after a crash: a panic hook now restores the terminal and logs the panic before printing it.
- Fixed an invalid `config.toml` silently falling back to defaults and later being overwritten: the file is now backed up to `config.toml.bak`, a localized error is shown at startup, and it is not overwritten until the user confirms via Options > Save setup.
- Failed writes of `config.toml` during startup are now logged instead of ignored.
- **Security:** Untrusted plugins now always have `pairee.fs` jailed to their own directory (read) and private data directory (read/write), regardless of Secure Mode; previously they had unrestricted filesystem access unless Secure Mode was enabled.
- **Security:** Secure Mode can no longer be disabled from Lua: the flag is captured in Rust instead of being read from the writable `pairee._secure_mode` global (now informational only).
- **Security:** Plugin path checks now normalize `..` and resolve symlinks through the nearest existing parent, closing a Secure Mode bypass via `..` on non-existent paths.
- **Security:** User menu commands expand `{f}` / `{p}` in a single pass, so a file name such as `;id;{p}` can no longer break shell quoting.
- **Security:** Registry plugin installs reject `[files]` entries that escape the plugin directory, validate plugin name/author, and verify each file's SHA-256 before writing it to disk.
- **Security:** The self-updater now requires the release `.sha256` asset, only downloads from `https://github.com/FittyAr/Pairee/releases/download/`, verifies the artifact in memory and installs from that same buffer (no shared, predictable `pairee_update` temp dir, no verify→extract race). SHA-256 now uses the `sha2` crate instead of a hand-rolled implementation.
- **Security:** Secure wipe no longer follows symbolic links or junctions: wiping a link removes only the link and never overwrites its target.
- **Security:** The 7-Zip helper downloaded on Windows is verified against a pinned SHA-256 before use; RAR/ISO extraction via external 7z now refuses to run when the archive cannot be listed and validated first.
- **Security:** Archive extraction (zip, tar.gz, 7z, RAR/ISO) never overwrites existing files (they are skipped and listed in the job results), never writes through symlinks already present in the destination, and aborts archives with more than 500,000 entries or 32 GiB of uncompressed data.
- **Security:** `config.toml` and other config files are written with `0600` permissions on Unix.
- **Security:** Secure Mode's plugin command blacklist normalizes names (case, path, `.exe`/`.cmd`/`.bat`… extensions, version suffixes) and now also blocks `wscript`, `cscript`, `mshta`, `rundll32`, `env`, `osascript`, `busybox`, more shells and interpreters, and command wrappers.
- **Security:** Plugin Lua states have a memory limit and a watchdog that aborts runaway scripts (e.g. infinite loops) instead of freezing Pairee.
- **Security:** `install.sh` / `install.ps1` fail on HTTP errors (`curl -fsSL`), validate the release tag and verify the downloaded archive against its `.sha256` before extracting.
- Fixed file panel views to properly honor the `git_enabled` setting when rendering Git status badges.
- Fixed silent no-op when attempting to delete the active checked-out branch in the Git branches tab.
- Fixed obscure failure when attempting to toggle commit amend on an empty repository without previous commits.
- Fixed duplicate placeholder replacement in branch merge confirmation dialog which mistakenly duplicated the source branch name instead of showing the target branch.
- Fixed duplicate placeholder replacement in commit reset confirmation dialog which hid the reset mode (Soft/Mixed/Hard).
- Fixed newly initialized repositories with zero commits showing `(detached HEAD)` instead of their initial branch name (e.g. `master` or `main`).
- Fixed push tags without configured remotes failing with obscure libgit2 error.
- **Security:** Secure Mode now uses a command allowlist for plugins instead of a name blacklist: a plugin may only run programs declared in its manifest, by name, resolved through `PATH` (no explicit or relative paths, no `.bat`/`.cmd`), and shells or interpreters are refused even if declared or reached through a symlink. A renamed binary can no longer slip through.
- On Windows, commands run from the command line, the user menu and apply-command now reach `cmd.exe` unmodified: quotes are no longer mangled, and file names containing `%VAR%`, `&`, `^`, `!`, `(`, `)` or quotes are passed literally instead of being expanded or split.
- **Security:** On Windows, opening a file with no association uses the system handler directly (`ShellExecuteW`) instead of `cmd /c start`, so the file name is never parsed by the shell.
- **Security:** Jailed plugin file operations (`pairee.fs` read, write, mkdir, remove, rename, copy, list) now run through a directory handle of the allowed root, so a folder swapped for a symlink or junction after the path check can no longer redirect them outside the jail.
- **Security:** Updates are now signed: the self-updater requires a minisign signature (`.minisig`) for the downloaded release asset in addition to the SHA-256 checksum and refuses to install if it is missing or invalid. The install scripts verify the signature when `minisign` is installed.



---

## [v0.8.1] - 2026-09-22

### Added

- Remote branch checkout support: selecting a remote branch in the Git panel now automatically creates the local branch, sets up upstream tracking, and checks it out.
- Remote branch merging support directly from the Git branches list.
- Granular staging support in Git panel with visual `[staged]` and `[staged+]` status indicators.
- Keybindings in Git Status tab: `a` to stage all changes, `A` to unstage all changes, `x`/`Delete` to discard changes with confirmation dialog, `i` to add path to `.gitignore`, and `X` to abort in-progress merges.
- Commit amend support (`Ctrl+A` toggle in commit dialog).
- Multi-remote management dialog in Git panel (`R` in Branches tab) to list, add, and remove remotes.
- Remote branch deletion directly from the Branches tab with confirmation dialog.
- Automatic upstream tracking configuration (`--set-upstream`) when pushing branches without an upstream.
- Branch creation from any selected commit in the Git Log tab (`b`/`n`).
- Tag creation from any selected commit in the Git Log tab (`t`).
- Cherry-pick and Revert operations with confirmation dialogs from the Git Log tab (`c` and `r`).
- Clipboard copy of commit SHA in Git Log tab (`y`).
- Incremental pagination and continuous loading when scrolling through commit history.
- Stash entry diff inspection (`d` in Stash tab).
- Stash clearing operation (`C` in Stash tab) with confirmation dialog.
- Include untracked files toggle (`Ctrl+U`) in Stash Save prompt.
- Interactive branch rebase (`b` in Branches tab) with confirmation dialog.
- Ahead and behind commit counter indicators (`[↑X ↓Y]`) displayed for local branches.
- Dedicated Tags tab in Git panel (`git_tab_tags`) displaying tags with target commit hash and annotated messages.
- Full tag management in Git panel: create (`n`), delete (`d`/`Delete`), checkout (`Enter`), and push tags to remote (`u`).
- Git repository initialization action (`GitInit`) accessible from top menu to initialize repositories directly in the active panel.
- Interactive Git Clone dialog (`GitClone`) accessible from top menu with HTTPS/SSH credentials and auto-derived target directories.
- Active Git branch indicator in navigation panel headers (e.g. `[git: main]`).
- File Git status indicators (`[M]`, `[A]`, `[?]`, `[D]`, `[!]`) and status color highlighting in all file navigation panels.

### Improved

- Dynamic remote resolution: git operations (`fetch`, `pull`, `push`) now automatically use the branch's tracked remote instead of hardcoding `origin`.
- Authentication expanded with support for `id_ed25519`, `id_ecdsa`, and standard Git Credential Manager for HTTPS.
- Git branch list now filters out symbolic `*/HEAD` references (such as `origin/HEAD`).
- Git checkout confirmation dialog now safely returns to the Git panel upon cancellation and refreshes file explorer panels upon success.
- Git commit prompt commits only staged files when files are explicitly staged.

### Changed

### Deprecated

### Removed

### Fixed

- Resolved Linux and cross-platform Clippy linter warnings (`collapsible_if`, `missing_transmute_annotations`, `io_other_error`, `let_and_return`) in CI pipelines.
- Addressed `cargo deny` advisory `RUSTSEC-2017-0008` (unmaintained transitive dependency `serial` pulled by `portable-pty`).

---

## [v0.8.0] - 2026-09-08

### Added

- Which-key overlay (`Ctrl+Shift+K`) lists the live keymap chords with labels, fuzzy-filters them, and runs the selected action with Enter. While a multi-key sequence is in progress, a prefix hint shows the remaining chords (same `keybinds` map, not a second keymap). Esc cancels the prefix.
- Copy path (`Ctrl+Shift+C`, Files menu, command palette) puts the hovered or tagged full path(s) on the OS clipboard. Command palette is bound as `Ctrl+Shift+P` in the shipped keymaps.
- Short threat model (`docs/THREAT_MODEL.md`) for plugins, SSH presets, updates, and the elevated helper.
- Structured tracing architecture (`tracing`, `tracing-subscriber`, `tracing-appender`) with rolling daily logs and environment filter control.
- Cross-platform POSIX and Windows signal handling (`SIGTERM`, `SIGINT`, `SIGHUP`, `SIGQUIT`, and console control events) for clean TUI restoration.
- CI test coverage workflow using `cargo-llvm-cov` to measure and report codebase test coverage.

- Plugin confirm, input, and which-key dialogs are real TUI overlays (`pairee.confirm` / `pairee.input` / `pairee.which`); Enter/Esc (and Y/N) reply to the waiting plugin.
- Typed `File` userdata (`name`, `path`, `url`, `size`, `is_dir`, `is_symlink`) and `pairee.cx` (cwd, hovered, selected) filled inside `pairee.sync`.
- Lua `File` metadata: `mime`, `mtime`, `is_hidden`, `is_exec` (Lua API **1.1.0**).
- Plugin filesystem extras: `mkdir`, `remove`, `rename`, `copy`, `read_dir`, and `file()` (File userdata).
- `pairee.Command` process builder with piped `Child` streaming (`write_all`, `read`, `wait_with_output`).
- Optional feature flags in Settings → System: SSH, plugins, and image preview (Git already had a toggle). Existing configs stay enabled.
- EN/ES translation keys are now complete and checked in CI (`scripts/check_translations.py`, `docs/i18n.md`).
- Parser smoke-fuzz tests (globs, descript.ion, plugin manifests, settings TOML) so junk input cannot panic.
- CI tests run on macOS as well as Linux and Windows.
- First-run keymap onboarding (Norton / Neovim / VS Code). Existing configs skip the dialog.
- Versioned Lua plugin API **v1.1.0** (`pairee._lua_api_version`, `docs/api/lua/`).
- CI acceptance plugins under `tests/plugin_acceptance/` (surface, fs, cx/utils, Command echo).
- `pairee.emit`, `pairee.notify`, and `pairee.file_cache` are callable functions (they were nested tables).
- Improvement tracking document at `docs/IMPROVEMENT_PLAN.md` with phased roadmap and progress checkboxes.
- Integration tests under `tests/` cover isolated temp workspace, Settings TOML roundtrip, shipped keymap presets, packaged EN/ES keys, AppState panel roots, and zip extract.
- Project-level `rustfmt.toml` and `clippy.toml` for consistent CI quality gates.
- Declared MSRV (`rust-version = "1.88"`, required by `tui-scrollbar`) and package metadata in `Cargo.toml`.
- Transfer Strategy backends (`local` / `ssh`) under `src/fs/transfer/backend/` with unified job submission.
- Command palette (`Ctrl+Shift+P`) to filter and run logical actions.
- Fractional scrollbars via `tui-scrollbar` (shared helper in `src/ui/scrollbar.rs`) on help, viewer/quickview, history lists, transfer panel, git panel, and related popups.
- Mouse drag and jump-to-click on scrollbars (`ScrollBarInteraction`, hit targets registered each frame, `EnableMouseCapture`).
- Unicode-aware file-name truncation helpers (`unicode-width` + `unicode-segmentation`) for panel columns.
- Bracketed paste: pasted text lands in the CLI or the open text prompt (rename, apply command, mkdir, …) as one string instead of fake keystrokes.
- Settings → Interface shows keymap validation (errors, warnings, bound count) and a “View keymap issues” overlay. Far-style `Gray+` / `Gray-` / `Gray*` aliases are documented as mapping to `Plus` / `-` / `*`.
- CI draw/resize smoke tests via `ratatui` TestBackend (not a substitute for a human pass on Windows Terminal / conhost / Linux).
- CI `cargo deny` job checks licenses, RustSec advisories, yanked crates, and crate sources (`deny.toml`).

### Improved

- Complete codebase modularization under the Single Responsibility Principle (SRP), bringing all `.rs` files across the project strictly below 300 lines (zero files exceed 300 lines).
- Decoupled `PopupType` variants into dedicated sub-structures (`GitPanelState`, `SshConnectPromptState`, `ConfigurationDialogState`, `PluginMenuState`, `CopyMovePromptState`) to keep overlay payloads modular, lightweight, and maintainable.
- Background Terminal (`command &`) and apply-command run on a real PTY (Windows ConPTY / Unix pty), so programs that check for a TTY can emit colors and use normal line buffering.
- Apply-command (`Ctrl+G`) opens the Terminal screen and shows captured stdout/stderr with ANSI colors, while the Transfer Engine still tracks progress.
- Command palette entries come from a shared `ActionDef` catalogue (`id` + category) instead of a second hardcoded name list.
- Native 7z extract/list uses maintained `sevenz-rust2` instead of unmaintained `sevenz-rust` (Zip-Slip sanitization kept).
- Command palette filters with Helix `nucleo-matcher` (fuzzy ranking) instead of substring `contains`.
- MSRV is **1.93** (required by `sevenz-rust2`).
- Clipboard writes use `arboard` instead of shelling out to `clip` / `xclip` / `wl-copy` (update “copy command” and Copy path).
- Background Terminal screen (`command &`) renders ANSI SGR colors from captured stdout/stderr instead of showing raw escape codes.
- Full-screen terminal clear runs inside the synchronized-update region, and only on resize or after a native TTY/admin restore.
- Keyboard enhancement and focus-change sequences are enabled only when the terminal supports them (Unix query; Windows CSI push), and popped only if they were pushed.
- File-association and CLI command lines split with POSIX `shlex` (quoted words) on Unix, and quote-aware tokens on Windows so `"C:\\Program Files\\App\\app.exe" %f` stays one program name.
- Keybindings engine rebuilt on the `keybinds` crate: invalid chords are rejected, duplicate chords across actions are rejected, and Norton/Neovim/VSCode presets load from validated TOML.
- TUI draw path uses synchronized updates and dirty-flag rendering to reduce flicker/glitches.
- Scroll indicators use theme colors and proportional thumbs instead of ratatui’s full-cell default.
- Clippy collapsible-if and related lint cleanups so `cargo clippy -- -D warnings` is green again.
- Clippy 1.98 cleanups (`useless_borrows_in_formatting`, `question_mark` in Lua `t()` lookup).
- CI `check` workflow now targets `master`/`main`, runs tests on Ubuntu and Windows, uses Node 24-aligned actions, and rejects crate-level `clippy::all` allows.
- Documentation index (`docs/README.md`) lists design docs with Implemented/Partial/Planned status.
- README (EN/ES) links corrected to `help/en` and `help/es`, project tree updated, plugin system no longer labeled as only planned.
- Transfer worker split into focused modules (scan, delete, copy, helpers) under `src/fs/transfer/worker/` using a facade orchestrator.
- Copy, move, and delete (including SSH) now use the Transfer Engine progress UI instead of the legacy modal-only path.
- Wipe, compress, extract, and apply-command jobs use the Transfer Engine queue and minimized panel (one consistent progress UX).
- Cooperative cancel for archive compress/extract (native formats check cancel between entries; external 7z is killed on cancel).

- Session state grouped into `PanelPair`, `HistoryState`, and `UpdateState` on `AppState`.
- Split oversized UI modules (transfer panel, history lists, settings actions, plugin dev options) into focused files.
- Internal F3 viewer split into `src/ui/viewer/{state,text,hex,image}.rs`.
- Overlay `PopupType` paste handling and plugin widgets live in focused files under `src/app/state/popup/`.
- Overlay dialogs live in `src/app/state/popup/` (`QuickViewDialog` boxed; config settings boxed) so `PopupType` is no longer a huge enum payload.
- Plugin updater, directory listing, and Settings split into focused modules.
- Dialogs use a `DialogStack` (`state.dialogs`) with replace/push/pop instead of a single `Option` popup.
- Background channels (search, SSH, terminal, updates, plugin progress) are polled in place instead of take/put-back.

### Changed

- Application logic lives in the `pairee` library crate; `src/main.rs` is a thin tokio entry so tests can `use pairee`.
- Replaced inherited rustc-style `.gitignore` with a Pairee-specific ignore list.
- Plugin manager core module renamed to `lifecycle` to avoid module-inception nesting.
- Long-running file jobs no longer use a separate modal progress dialog.

### Deprecated

### Removed

- Local temporary `.tmp*` workspaces and the vendored local `example/` reference tree from the working tree (still ignored by git).
- Legacy `ops_worker` spawn stack, `progress_rx` / `BackgroundOpContext`, and the `CopyProgress` modal UI.

### Fixed

- Trusted Lua plugins load again (`StdLib::ALL_SAFE` instead of `ALL`, which rejected `debug` under `new_with`).
- Clippy is enforced without `#![allow(clippy::all)]` in `src/main.rs`.
- Outdated status banners on transfer-engine and plugin-system design docs.

---

## [v0.7.2] - 2026-08-06

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

---

## [v0.7.1] - 2026-07-20

### Added

### Improved

### Changed

### Deprecated

### Removed

### Fixed

- Fixed PowerShell command execution syntax in GitHub Actions workflow (`.github/workflows/release.yml`) during MSIX packaging.
- Fixed directory tree removal when moving folders in the background Transfer Engine so that empty source subdirectories and root folders are completely cleaned up.

---

## [v0.7.0] - 2026-07-20

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

---

## [v0.6.1] - 2026-06-27

### Added

- A rule in `.agents/AGENTS.md` to enforce checking and running workspace customization skills automatically.
- WinGet installation helper submenu to `run.bat` and `run.sh` for auto-detecting, forcing architecture installs (x64/arm64), upgrading, and uninstalling Pairee.
- Comprehensive `docs/winget-submission-guide.md` documenting the manual first-time submission, PR troubleshooting, and GitHub Actions release automation.
- Detailed `docs/technical/microsoft-store-publishing.md` explaining how to package and publish Pairee to the Microsoft Store as an MSIX package without a paid certificate.
- Microsoft Store (MSIX) Developer Menu submenu in `run.bat` and `run.sh` for local packaging, test certificate generation, signing, and installation of MSIX packages.
- MSIX manifest template (`AppxManifest.xml`) and asset placeholders under `manifests/msix/`.
- Automatic MSIX packaging and version bumping for Windows targets integrated into the `.github/workflows/release.yml` release workflow.
- Desktop shortcut creation option and Windows Control Panel uninstallation icon support in the Inno Setup installer script.
- Custom Windows resource compilation (`manifests/windows/pairee.rc`) to embed the new multi-resolution icon (`pairee.ico`) directly inside the built `pairee.exe` executable.
- Linux desktop launcher entry (`manifests/linux/pairee.desktop`) and SVG/PNG app icon packaging in `Cargo.toml` for Debian and RPM packages.

### Changed

- Updated local WinGet package manifests for v0.6.0: corrected the license to `GPLv3`, added the `arm64` installer architecture details with valid SHA-256 hashes, and added the Spanish (`es-ES`) translation locale.
- Configured Windows targets (MSVC) in Cargo to statically link the C runtime library (CRT), eliminating runtime dependencies on `VCRUNTIME140.dll`.

### Improved

- Enlarged the self-update popup, added line wrapping for release notes, styled markdown headers, and implemented vertical scrolling with a scrollbar.
- Cached the installation method detection using `OnceLock` to prevent TUI thread freezes during self-update rendering and activation.

### Fixed

- Output duplication in `scripts/extract_changelog.sh` that caused duplicated release descriptions on GitHub releases.
- WinGet validation error (STATUS_DLL_NOT_FOUND) resolved by adding VC++ Redistributable package dependencies in the manifest.
- Syntax errors and rendering bugs in `run.bat` helper script resolved.

---

## [v0.6.0] - 2026-06-26

### Added

- Process name filtering in the task list dialog with live list reordering and deactivated color styling for non-matching entries.
- Interactive "About" dialog (accessible via options menu or shortcut) displaying license, project details, and dependencies with scrolling support.
- Automated version bumping and release note extraction scripts to automate release steps.
- Dynamic build-time metadata tracking (target platform, Git commit hash, build profile) integrated into the binary using a new `build.rs` script.
- Structured GitHub issue templates (bug reports, feature requests) and config templates to standardize community feedback.
- Community health documents including `CODE_OF_CONDUCT.md` and `CONTRIBUTING.md`.
- Workspace customization folder `.agents/` with automated AI skills (`localize-helper`, `settings-helper`, `changelog-helper`) and guidelines `AGENTS.md`.
- Parameterized `Dockerfile.namespace` for containerized compiling and testing in namespace environments.
- GitHub Actions validation workflow (`check.yml`) to automatically test code, formatting, and lints on pull requests.

### Changed

- Decoupled and modularized individual popup prompt rendering logic into dedicated files.
- Consolidated and moved workspace AI instructions from root `agents.md` to `.agents/AGENTS.md`.

### Fixed

- MkDir dialog: typed characters now immediately reflect in the input field.
- Rename/Move dialog: `to` input field is no longer empty and accepts text; first button now shows the correct `Rename` label.
- Copy dialog: destination `to` path is now correctly pre-filled.
- Update popup: `Esc` now dismisses the download progress dialog and no longer locks the UI.

### Improved

- All dialog popups now use fixed-height layouts, preventing input fields, checkboxes, and buttons from being cut off in standard terminal sizes (80×24).

---

## [v0.5.1] - 2026-06-25

### Added

- Cross-platform installation detection: Pairee detects how it was installed (installer, portable, package manager) and issues the appropriate upgrade command.
- Terminal key diagnostics tool for debugging input event handling.
- User menu system: users can now define custom menus with their own commands.
- Expanded panel view modes: additional display options with custom descriptions and file metadata.

### Changed

- Modularized file operation prompt UIs: each prompt type now lives in its own module under `src/ui/popup/prompts/file_ops/`.
- Modularized menu, popup, screen input handler, and main app loop into sub-modules for improved maintainability.
- CI: bumped `actions/checkout` to v7 and `action-gh-release` to v3.

---

## [v0.5.0] - 2026-06-25

### Added

- Automated self-update system: Pairee checks for new releases on GitHub, downloads, and installs updates with a progress UI.
- Comprehensive English and Spanish user documentation covering keyboard shortcuts, SSH/SFTP, Git integration, and configuration.

### Changed

- Filesystem deletion is now recursive with elevated operations support.
- Interactive configuration dialog management added.

---

## [v0.4.1] - 2026-06-24

### Added

- Configuration dialog system with interactive settings management UI.
- User-defined menus with custom shell commands and process restart support.
- Sort mode menu with configurable sort actions.
- Multiple key bindings support for a single custom action (comma-separated).
- Integrated Git workflow: repository status, log viewer, commit management, and dedicated UI panels.

### Changed

- Pre-flight Git authentication checks added to version bump scripts to prevent failed pushes.
- Help documentation UI redesigned with a split-pane layout, scrollbar, and improved keyboard navigation.
- Cross-platform elevated privilege handling refactored for filesystem operations.
- Dependencies bumped to v0.4.0 baseline.

---

## [v0.3.2] - 2026-06-17

### Added

- SSH connection presets with navigation and management support in the connection popup.
- SSH disconnect functionality with a menu option.
- Multiple panel rendering modes for the file explorer.
- Background file copy worker with progress tracking and admin privilege escalation support.

### Fixed

- `ssh2` dependency restricted to correct platform-specific targets; vendored OpenSSL enabled for non-Windows builds.

### Changed

- Filesystem operation error messages are now localized.
- File system operations refactored into dedicated service modules (`delete`, `mkdir`, `rename_move`).
- Resource and localization path resolution enhanced with recursive directory searching.
- Documentation directory added to all installer configurations.

---

## [v0.2.2] - 2026-06-15

### Added

- Filesystem operations with interactive user confirmation dialogs and progress tracking.
- File operation dialogs: rename/move, copy, delete, link, wipe, compress.
- Confirmation dialogs and modular file system operations logic.
- Localization system for all user-facing strings.

---

## [v0.2.1] - 2026-06-15

### Added

- Editor and viewer input handling with file operation integrations.
- Core application state management with modular initialization.
- UI prompts for help, file operations, and localized configuration management.
- System helper module for process management, drive enumeration, bookmarks, and tree navigation.

---

## [v0.2.0] - 2026-06-11

### Added

- GitHub Actions workflow for multi-platform binary releases (Linux GNU, Linux musl, Windows x64/arm64).
- Extensible keybinding resolver and registry system for mapping application commands.
- X11 modifier polling and input handling modules.
- Localization support infrastructure.

---

## [v0.1.7] - 2026-06-11

### Added

- Initial dual-panel TUI file manager core with `ratatui` + `crossterm`.
- Basic directory listing, navigation, and focus management.
- Application event loop with resize handling.
- Configuration loading from TOML files.
- Theme system with color and style definitions.

---

## [v0.1.6] - 2026-06-10

### Fixed

- `cargo-deb` path validation errors.
- Inno Setup `iscc.exe` argument translation on Windows CI runner.
- Cross-compilation pipeline errors for musl targets.

---

## [v0.1.2] - 2026-06-10

### Added

- Automated CI/CD release pipeline with version bumping scripts.
- Inno Setup installer configuration for Windows.
- `cargo-deb` and `cargo-generate-rpm` packaging for Linux.
- Version bump scripts (`bump_version.ps1` / `bump_version.sh`).

---

## [v0.1.1] - 2026-06-10

### Added

- Initial project skeleton with `main.rs` entry point and module layout.
- `Cargo.toml` with core dependencies: `ratatui`, `crossterm`, `tokio`, `serde`, `directories`, `anyhow`, `thiserror`, `log`, `simplelog`.
