## [Unreleased]

### Added

- Folder shortcuts can now be assigned: **Commands → Folder shortcuts** lists slots `Ctrl+Alt+1` … `Ctrl+Alt+9`; `Ins`/`Space` (or the slot digit) assigns the current folder and `Del` clears it. Shortcuts are saved in `bookmarks.toml` in the config folder.
- The directory hotlist (`Ctrl+\`, also **Commands → Directory hotlist**) is now persistent: `Ins`/`+` adds the current folder, `Del`/`-` removes an entry. Default entries are localized.
- `Alt+G` now opens the Git panel in every built-in keymap, as the menu already advertised.
- Built-in keymaps now pick up newly shipped bindings even when an older copy of the keymap file exists in the config folder (user-defined chords always win).
- Editor and viewer honor the "Tab size" settings when displaying tab characters.
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

### Improved

- Release builds now use thin LTO, a single codegen unit and stripped symbols for a smaller, faster binary.
- Untracked file badges and labels in Git panel now render in Magenta for consistent contrast and readability across dark backgrounds.
- Enhanced scroll behavior in the Git diff viewer for short files.
- Centralized all Git operation error alerts, conflict notifications, confirmation prompts, and buttons into localization catalogs with zero hardcoding.

### Deprecated

### Removed

- Configuration dialog options that had no effect (file descriptions, info panel details, dialog/command-line editing, most editor/viewer options, plugin manager flags and some confirmations) are hidden until implemented; their stored values are kept. See `docs/technical/settings-audit.md`.
### Fixed

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


