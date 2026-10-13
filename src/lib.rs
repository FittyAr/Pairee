//! Pairee library surface. The `pairee` binary is a thin `#[tokio::main]` wrapper around [`run`].
//!
//! Modules stay crate-private so internal `pub` items are not a public API.
//! Integration tests and benches import the re-exports below.

mod app;
mod config;
mod diff;
mod fs;
mod git;
mod keybindings;
mod launch_args;
mod lock;
mod logging;
mod plugin;
mod run;
mod shell;
mod syntax;
mod terminal;
mod text_width;
mod ui;
mod update;

/// Parser entry points for `fuzz/` (feature `fuzzing`) and `test_fuzz`.
#[cfg(any(test, feature = "fuzzing"))]
#[doc(hidden)]
pub mod fuzz_api;
#[cfg(test)]
mod smoke_tests;
#[cfg(test)]
mod test_fuzz;
#[cfg(test)]
mod test_harness;

pub use app::AppState;
pub use app::state::ActivePanel;
pub use config::settings::Settings;
pub use run::run;

/// Filesystem engines exercised by `tests/fs_engines.rs`: the duplicate
/// finder (also behind the Find duplicates view) and the remote `Vfs`
/// backends (library API until their screens come).
pub use fs::{dupes, remote, vfs};
