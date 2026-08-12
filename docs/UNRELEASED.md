## [Unreleased]

### Added

- Improvement tracking document at `docs/IMPROVEMENT_PLAN.md` with phased roadmap and progress checkboxes.
- Integration smoke tests under `tests/` for isolated temp workspace and settings TOML roundtrip.
- Project-level `rustfmt.toml` and `clippy.toml` for consistent CI quality gates.
- Declared MSRV (`rust-version = "1.85"`) and package metadata in `Cargo.toml`.

### Improved

- CI `check` workflow now targets `master`/`main`, runs tests on Ubuntu and Windows, uses Node 24-aligned actions, and rejects crate-level `clippy::all` allows.
- Documentation index (`docs/README.md`) lists design docs with Implemented/Partial/Planned status.
- README (EN/ES) links corrected to `help/en` and `help/es`, project tree updated, plugin system no longer labeled as only planned.

### Changed

- Replaced inherited rustc-style `.gitignore` with a Pairee-specific ignore list.
- Plugin manager core module renamed to `lifecycle` to avoid module-inception nesting.

### Deprecated

### Removed

- Local temporary `.tmp*` workspaces and the vendored local `example/` reference tree from the working tree (still ignored by git).

### Fixed

- Clippy is enforced without `#![allow(clippy::all)]` in `src/main.rs`.
- Outdated status banners on transfer-engine and plugin-system design docs.
