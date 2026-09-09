## [Unreleased]

### Added

### Improved

### Changed

### Deprecated

### Removed

### Fixed

- Resolved Linux and cross-platform Clippy linter warnings (`collapsible_if`, `missing_transmute_annotations`, `io_other_error`, `let_and_return`) in CI pipelines.
- Addressed `cargo deny` advisory `RUSTSEC-2017-0008` (unmaintained transitive dependency `serial` pulled by `portable-pty`).
