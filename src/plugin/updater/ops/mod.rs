//! Plugin updater operations (installation, updates, queries, verification).

pub mod install;
pub mod manage;
pub mod query;

pub use install::install;
pub use manage::{pin, remove, update, verify};
pub use query::{check_updates, list_installed, search, show_info};
