//! Developer Tools (F11 Tab 2): init, lint, package, install, and submit
//! workflow for plugin authors.
//!
//! The original 924-line god file was refactored per AGENTS.md §1
//! (Single Responsibility Principle) into focused submodules:
//!
//! - `paths` — path resolution helpers (`dev_plugin_dir`,
//!   `packaged_plugin_dir`).
//! - `progress` — progress reporting (`begin_dev_op`, `progress_status`,
//!   `dev_op_running`).
//! - `options` — the `handle_dev` key-event handler that dispatches the
//!   nine developer options.
//! - `select_popup` — the `handle_select_popup` handler for the
//!   "Select active plugin" modal.
//!
//! This `mod.rs` re-exports the public surface so the existing
//! `crate::app::input_popup::plugin_menu::dev::{handle_dev,
//! handle_select_popup}` path keeps working unchanged.

pub mod options;
pub mod paths;
pub mod progress;
pub mod select_popup;

pub use options::handle_dev;
pub use select_popup::handle_select_popup;

// The parent module's `reload_installed_plugins` helper is needed by
// every option handler; expose it at the `dev` level so the submodules
// do not need to reach outside their own scope.
pub(crate) use super::reload_installed_plugins;
// Internal re-exports for the option handlers below. These are crate-
// private (only used inside the `dev` module subtree).
pub(crate) use paths::{DEV_OPT_COUNT, move_active_panel_to};
