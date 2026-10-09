## [Unreleased]

### Added
- `pairee --version` (`-V`) prints the installed version.
- macOS builds (Intel and Apple silicon) on every release; the built-in updater picks them up.

### Improved

### Changed
- Release tooling: `cargo xtask release` replaces the PowerShell and Bash bump scripts and updates every dependency to its latest compatible version before each release. Releases are built only from commits that pass every check, every package is smoke-tested, and a draft is published after approval. A pre-push hook (`cargo xtask install-hooks`) runs rustfmt and the spell check locally.

### Deprecated

### Removed

### Fixed
