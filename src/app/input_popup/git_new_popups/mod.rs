//! Input handlers for Git Diff, Branch/Stash Prompts, and Confirmation popups.

mod clone_dialog;
mod common;
mod confirm;
mod diff;
mod prompts;
mod remote_manage;

pub use clone_dialog::handle_clone;
pub use common::{close_to, restore_previous_and_refresh, run_and_report};
pub use confirm::handle_confirm_action;
pub use confirm::open_confirm;
pub use diff::handle_diff;
pub use diff::open_diff;
pub use prompts::{handle_prompt, open_name_prompt};
pub use remote_manage::{handle_remote_add, handle_remote_manage};
