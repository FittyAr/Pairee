## [Unreleased]

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
