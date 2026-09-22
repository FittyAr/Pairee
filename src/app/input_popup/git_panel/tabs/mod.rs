//! Key action handlers for each tab in GitPanel (Status, Log, Branches, Stash).

pub mod branch_stash;
pub mod status_log;
pub mod tags;

pub use branch_stash::{handle_branch_tab, handle_stash_tab};
pub use status_log::{handle_log_tab, handle_status_tab};
pub use tags::handle_tag_tab;
