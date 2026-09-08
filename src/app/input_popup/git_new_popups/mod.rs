//! Input handlers for Git Diff, Branch/Stash Prompts, and Confirmation popups.

mod common;
mod confirm;
mod diff;
mod prompts;

pub use confirm::handle_confirm_action;
pub use diff::handle_diff;
pub use prompts::handle_prompt;
