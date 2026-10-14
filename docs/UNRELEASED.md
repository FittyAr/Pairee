## [Unreleased]

### Added

### Improved

### Changed

- Release files no longer carry a second copy named with the Rust target triple (`pairee-vX.Y.Z-x86_64-unknown-linux-musl.tar.gz` and so on): only the platform names (`pairee-X.Y.Z-linux-x64-musl.tar.gz`) remain. 0.11.0 was the last release with both, so Pairee 0.10 or older must update to 0.11.0 (or reinstall) before it can update further. The release build jobs are named by platform too.
### Deprecated

### Removed

### Fixed
