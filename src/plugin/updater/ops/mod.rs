//! Plugin updater operations (installation, updates, queries, verification).

mod files;
pub mod install;
mod install_guard;
pub mod manage;
pub mod query;

pub use install::install;
#[cfg(test)]
pub(crate) use install_guard::validate_identifier;
pub use manage::{pin, remove, update, verify};
pub use query::{check_updates, list_installed, search, show_info};
