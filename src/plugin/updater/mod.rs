pub mod lockfile;
pub mod ops;
pub mod registry;
pub mod types;

pub use lockfile::{read_lockfile, write_lockfile};
pub use ops::{
    check_updates, install, list_installed, pin, remove, search, show_info, update, verify,
};
pub use registry::fetch_index;
pub use types::{PinnedPlugin, RegistryIndex, RegistryPlugin};
