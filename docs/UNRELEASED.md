## [Unreleased]

### Added

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

### Fixed

- Fixed crashes in the built-in editor, the Git commit prompt and editor search when typing or moving the cursor over non-ASCII text (e.g. `ñ`, `á`, emoji, combining accents); cursor movement, Backspace and Delete now operate on whole grapheme clusters via a shared text-input helper.
- Fixed the terminal being left in raw mode / alternate screen with mouse capture after a crash: a panic hook now restores the terminal and logs the panic before printing it.
- Fixed an invalid `config.toml` silently falling back to defaults and later being overwritten: the file is now backed up to `config.toml.bak`, a localized error is shown at startup, and it is not overwritten until the user confirms via Options > Save setup.
- Failed writes of `config.toml` during startup are now logged instead of ignored.
- **Security:** Untrusted plugins now always have `pairee.fs` jailed to their own directory (read) and private data directory (read/write), regardless of Secure Mode; previously they had unrestricted filesystem access unless Secure Mode was enabled.
- **Security:** Secure Mode can no longer be disabled from Lua: the flag is captured in Rust instead of being read from the writable `pairee._secure_mode` global (now informational only).
- **Security:** Plugin path checks now normalize `..` and resolve symlinks through the nearest existing parent, closing a Secure Mode bypass via `..` on non-existent paths.
- **Security:** User menu commands expand `{f}` / `{p}` in a single pass, so a file name such as `;id;{p}` can no longer break shell quoting.
- **Security:** Registry plugin installs reject `[files]` entries that escape the plugin directory, validate plugin name/author, and verify each file's SHA-256 before writing it to disk.
- Fixed file panel views to properly honor the `git_enabled` setting when rendering Git status badges.
- Fixed silent no-op when attempting to delete the active checked-out branch in the Git branches tab.
- Fixed obscure failure when attempting to toggle commit amend on an empty repository without previous commits.
- Fixed duplicate placeholder replacement in branch merge confirmation dialog which mistakenly duplicated the source branch name instead of showing the target branch.
- Fixed duplicate placeholder replacement in commit reset confirmation dialog which hid the reset mode (Soft/Mixed/Hard).
- Fixed newly initialized repositories with zero commits showing `(detached HEAD)` instead of their initial branch name (e.g. `master` or `main`).
- Fixed push tags without configured remotes failing with obscure libgit2 error.


