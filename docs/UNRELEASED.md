## [Unreleased]

### Added

- Support for untracked files in the Git diff viewer, allowing inspection of newly added files with full contents.
- Home and End key navigation in the unified Git diff viewer.
- Context-sensitive empty list notifications across all Git panel tabs (Log, Branches, Stash, and Tags).
- Full suite of unit tests for file diffs (staged, unstaged, untracked), commit diffs, and stash diffs.

### Improved

- Untracked file badges and labels in Git panel now render in Magenta for consistent contrast and readability across dark backgrounds.
- Enhanced scroll behavior in the Git diff viewer for short files.
- Centralized all Git operation error alerts, conflict notifications, confirmation prompts, and buttons into localization catalogs with zero hardcoding.

### Changed

### Deprecated

### Removed

### Fixed

- Fixed file panel views to properly honor the `git_enabled` setting when rendering Git status badges.

