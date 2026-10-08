## [Unreleased]

### Added

- **Commands → Synchronize folders** (Total Commander "Synchronize dirs"): compares both panel folders recursively in the background (size and modification time with the configurable tolerance, optionally file contents with the transfer hash algorithm; filter mask with include/exclude globs; hidden files can be ignored), lists every difference with an action per item (copy →, copy ←, delete, skip) and totals of files and bytes, and applies the plan as Transfer Engine copy/delete jobs. Directions: left → right, right → left, both ways (newer wins) and mirror (deletes extras on the right, after an explicit confirmation).
- Folder shortcuts can now be assigned: **Commands → Folder shortcuts** lists slots `Ctrl+Alt+1` … `Ctrl+Alt+9`; `Ins`/`Space` (or the slot digit) assigns the current folder and `Del` clears it. Shortcuts are saved in `bookmarks.toml` in the config folder.
- The directory hotlist (`Ctrl+\`, also **Commands → Directory hotlist**) is now persistent: `Ins`/`+` adds the current folder, `Del`/`-` removes an entry. Default entries are localized.
- `Alt+G` now opens the Git panel in every built-in keymap, as the menu already advertised.
- Built-in keymaps now pick up newly shipped bindings even when an older copy of the keymap file exists in the config folder (user-defined chords always win).
- Editor and viewer honor the "Tab size" settings when displaying tab characters.
- Built-in editor: undo/redo (`Ctrl+Z` / `Ctrl+Y`, typing runs are one step), save as (`Shift+F2`), `Tab` key, `Home`/`End` and `Ctrl+Home`/`Ctrl+End`, horizontal scrolling for long lines, and `F6` in the viewer opens the file in the editor. Line endings (LF/CRLF), the final newline and the UTF-8 BOM are preserved on save. Saving asks before overwriting a file that another program changed or an existing "save as" target. Files that are not valid UTF-8 or larger than 64 MiB are refused with an explanation instead of being corrupted.
- Editor settings that had no effect now work and are back in **Options → Configuration → Editor/Viewer**: expand tabs, auto indent, show line numbers, cursor at the end, lock editing of read-only files and warn when opening read-only files.
- Panel visibility, view mode, sort order and long-names mode are restored at startup from the last **Save setup**.
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
- Plugins can declare the programs they need in `manifest.toml` (`[permissions] commands = [...]`); the Plugin Manager shows them in the plugin details.

### Improved

- **Commands → Compare folders** is now recursive (a folder on both sides is reported as different when anything inside differs) and runs in the background with a progress popup; `Esc` cancels it.
- The Screens menu and the development-plugin picker now use the theme's selection colors, like the other list popups.
- Text fields in dialogs (make folder, rename, copy/move destination, compress, apply command, describe, link, filters, select group) now share one input box: the cursor can be moved with Left/Right/Home/End, Delete removes the character under it, `Ctrl+V` pastes from the clipboard, and non-ASCII text is edited by whole characters.
- Panels are now read in the background: the directory listing, SFTP listing, Git status and free space no longer block the interface, the panel title shows "⟳ Loading…" while a read is in progress, and the cursor and selection are kept when the new listing arrives. Entering a directory rereads only that panel, and Git status is computed only for the listed directory instead of the whole repository.
- Git fetch, pull, push, push tags, remote branch deletion and clone now run in the background with a progress popup (objects and bytes transferred); `Esc` cancels the operation. The Left/Right menus no longer open the repository while drawing.
- Quick view loads previews in the background after the cursor settles (no more stalls when scrolling through large files), caches recent previews per file version, and reads at most 16 MiB per file. The F3 viewer also loads in the background, reads at most 64 MiB (larger files are shown truncated with a notice) and shows the error instead of an empty viewer when the file cannot be read.
- Detailed, Owners and Links views no longer read file attributes on every repaint, and the free-space footer no longer queries the disk on every repaint; both are refreshed with the listing.
- Release builds now use thin LTO, a single codegen unit and stripped symbols for a smaller, faster binary.
- Untracked file badges and labels in Git panel now render in Magenta for consistent contrast and readability across dark backgrounds.
- Enhanced scroll behavior in the Git diff viewer for short files.
- Centralized all Git operation error alerts, conflict notifications, confirmation prompts, and buttons into localization catalogs with zero hardcoding.
- Internal: the "do not overwrite an invalid `config.toml` until confirmed" lock and the startup load error now live in `AppConfig` (`ConfigLoadState`) instead of process-wide globals.

### Deprecated

### Removed

- External editor support: Pairee edits files only with its built-in editor. F4 and every other "edit" entry point always open the internal editor. The `editor_use_external` and `default_editor` settings are gone (old `config.toml` files that still contain them load fine), and the default file associations no longer launch `notepad`/`nano` for text files; untouched old default rules of that kind are removed from `associations.toml` on load.
- Settings that had no effect (file descriptions, info panel details, dialog/command-line editing, editor code pages and blocks, most viewer options, plugin manager flags, `git_auto_detect`, `transfer_engine_enabled` and some confirmations) were removed from the configuration, the dialog and the manuals. Old `config.toml` files that still contain them load fine; the keys are dropped on the next save. See `docs/technical/settings-audit.md`.
### Fixed

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


