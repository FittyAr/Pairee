//! Platform shell helpers: quoting of untrusted values, building the shell
//! command for a user command line, and opening files with the system
//! handler. Every place that hands a string to `sh -c` / `cmd.exe /C` goes
//! through this module so the quoting rules live in one place.

#[cfg(windows)]
mod cmd_env;
mod command;
mod open;
mod quote;

pub use command::{shell_command, shell_invocation};
pub use open::open_with_system_handler;
pub use quote::{expand_placeholders, quote_cmd, quote_native, quote_path, quote_posix};
