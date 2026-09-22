## [Unreleased]

### Added

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

- Untracked file badges and labels in Git panel now render in Magenta for consistent contrast and readability across dark backgrounds.
- Enhanced scroll behavior in the Git diff viewer for short files.
- Centralized all Git operation error alerts, conflict notifications, confirmation prompts, and buttons into localization catalogs with zero hardcoding.

### Deprecated

### Removed

### Fixed

- Fixed file panel views to properly honor the `git_enabled` setting when rendering Git status badges.
- Fixed silent no-op when attempting to delete the active checked-out branch in the Git branches tab.
- Fixed obscure failure when attempting to toggle commit amend on an empty repository without previous commits.
- Fixed duplicate placeholder replacement in branch merge confirmation dialog which mistakenly duplicated the source branch name instead of showing the target branch.
- Fixed duplicate placeholder replacement in commit reset confirmation dialog which hid the reset mode (Soft/Mixed/Hard).
- Fixed newly initialized repositories with zero commits showing `(detached HEAD)` instead of their initial branch name (e.g. `master` or `main`).
- Fixed push tags without configured remotes failing with obscure libgit2 error.


