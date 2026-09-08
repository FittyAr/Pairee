//! Pairee library surface. The `pairee` binary is a thin `#[tokio::main]` wrapper around [`run`].
//!
//! Modules stay crate-private so internal `pub` items are not a public API.
//! Integration tests and benches import the re-exports below.

mod app;
mod config;
mod fs;
mod git;
mod keybindings;
mod logging;
mod plugin;
mod run;
mod terminal;
mod ui;
mod update;

#[cfg(test)]
mod test_fuzz;

pub use app::AppState;
pub use config::settings::Settings;
pub use run::run;
