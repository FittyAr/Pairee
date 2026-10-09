pub mod acceptance;
pub mod command_policy;
pub mod commands_decl;
pub mod developer_tool;
pub mod hooks;
pub mod installed;
pub mod keyspec;
pub mod limits;
pub mod loader;
pub mod manager;
pub mod registry;
pub mod runtime;
pub mod sandbox;
pub mod updater;

pub use manager::{PluginManager, process_plugin_requests};
