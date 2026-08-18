## [Unreleased]

### Added

- Improvement tracking document at `docs/IMPROVEMENT_PLAN.md` with phased roadmap and progress checkboxes.
- Integration smoke tests under `tests/` for isolated temp workspace, settings TOML roundtrip, and transfer filesystem contracts.
- Project-level `rustfmt.toml` and `clippy.toml` for consistent CI quality gates.
- Declared MSRV (`rust-version = "1.88"`, required by `tui-scrollbar`) and package metadata in `Cargo.toml`.
- Transfer Strategy backends (`local` / `ssh`) under `src/fs/transfer/backend/` with unified job submission.
- Command palette (`Ctrl+Shift+P`) to filter and run logical actions.
- Fractional scrollbars via `tui-scrollbar` (shared helper in `src/ui/scrollbar.rs`) on help, viewer/quickview, history lists, transfer panel, git panel, and related popups.
- Mouse drag and jump-to-click on scrollbars (`ScrollBarInteraction`, hit targets registered each frame, `EnableMouseCapture`).
- Unicode-aware file-name truncation helpers (`unicode-width` + `unicode-segmentation`) for panel columns.

### Improved

- Keybindings engine rebuilt on the `keybinds` crate: invalid chords are rejected, duplicate chords across actions are rejected, and Norton/Neovim/VSCode presets load from validated TOML.
- TUI draw path uses synchronized updates and dirty-flag rendering to reduce flicker/glitches.
- Scroll indicators use theme colors and proportional thumbs instead of ratatui’s full-cell default.
- Clippy collapsible-if and related lint cleanups so `cargo clippy -- -D warnings` is green again.
- CI `check` workflow now targets `master`/`main`, runs tests on Ubuntu and Windows, uses Node 24-aligned actions, and rejects crate-level `clippy::all` allows.
- Documentation index (`docs/README.md`) lists design docs with Implemented/Partial/Planned status.
- README (EN/ES) links corrected to `help/en` and `help/es`, project tree updated, plugin system no longer labeled as only planned.
- Transfer worker split into focused modules (scan, delete, copy, helpers) under `src/fs/transfer/worker/` using a facade orchestrator.
- Copy, move, and delete (including SSH) now use the Transfer Engine progress UI instead of the legacy modal-only path.
- Wipe, compress, extract, and apply-command jobs use the Transfer Engine queue and minimized panel (one consistent progress UX).
- Cooperative cancel for archive compress/extract (native formats check cancel between entries; external 7z is killed on cancel).

- Session state grouped into `PanelPair`, `HistoryState`, and `UpdateState` on `AppState`.
- Split oversized UI modules (transfer panel, history lists, settings actions, plugin dev options) into focused files.
- Overlay dialogs live in `src/app/state/popup/` (`QuickViewDialog` boxed; config settings boxed) so `PopupType` is no longer a huge enum payload.
- Plugin updater, directory listing, and Settings split into focused modules.
- Dialogs use a `DialogStack` (`state.dialogs`) with replace/push/pop instead of a single `Option` popup.
- Background channels (search, SSH, terminal, updates, plugin progress) are polled in place instead of take/put-back.

### Changed

- Replaced inherited rustc-style `.gitignore` with a Pairee-specific ignore list.
- Plugin manager core module renamed to `lifecycle` to avoid module-inception nesting.
- Long-running file jobs no longer use a separate modal progress dialog.

### Deprecated

### Removed

- Local temporary `.tmp*` workspaces and the vendored local `example/` reference tree from the working tree (still ignored by git).
- Legacy `ops_worker` spawn stack, `progress_rx` / `BackgroundOpContext`, and the `CopyProgress` modal UI.

### Fixed

- Clippy is enforced without `#![allow(clippy::all)]` in `src/main.rs`.
- Outdated status banners on transfer-engine and plugin-system design docs.
