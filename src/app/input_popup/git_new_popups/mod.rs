//! Input handlers for Git Diff, Branch/Stash Prompts, and Confirmation popups.

mod clone_dialog;
mod common;
mod confirm;
mod diff;
mod prompts;
mod remote_manage;

pub use common::{close_to, refresh_panel_on_top, run_and_report};
pub use confirm::open_confirm;
pub use diff::open_diff;
pub use prompts::open_name_prompt;
