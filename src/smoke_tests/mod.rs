//! End-to-end smoke scenarios over a temporary sandbox (G.5), driven by
//! the [`crate::test_harness::Harness`]. They replace the manual pass as far
//! as a `TestBackend` allows; what still needs a human is listed in
//! `docs/IMPROVEMENT_PLAN.md`.

mod archives;
mod bookmarks;
mod editor;
mod file_clipboard;
mod file_ops;
mod git_panel;
mod navigation;
mod panel_search;
mod polish;
mod refresh_resize;
mod screen_keymaps;
mod selection_jumps;
mod sizes_sync;
mod tabs_session;
mod transfer;
mod typing_modes;
mod vfs_followups;
mod viewer;
