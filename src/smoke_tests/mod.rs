//! End-to-end smoke scenarios over a temporary sandbox (G.5), driven by
//! the [`crate::test_harness::Harness`]. They replace the manual pass as far
//! as a `TestBackend` allows; what still needs a human is listed in
//! `docs/IMPROVEMENT_PLAN.md`.

mod archives;
mod bookmarks;
mod command_line;
mod editor;
mod editor_search;
mod file_clipboard;
mod file_ops;
mod find_file;
mod git_panel;
mod mouse;
mod navigation;
mod panel_render;
mod panel_scroll;
mod panel_search;
mod plugin_keys;
mod polish;
mod refresh_resize;
mod screen_keymaps;
mod selection_jumps;
mod shortcuts_modal;
mod sizes_sync;
mod tabs_session;
mod terminal_screens;
mod tiny_terminal;
mod transfer;
mod typing_modes;
mod vfs_followups;
mod viewer;
