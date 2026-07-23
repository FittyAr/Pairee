//! M3 process module: `Command` builder, `Child`, `Output`,
//! `Status`, `Stdio` constants, `Access`, `Fd` userdata.
//!
//! These types are the foundation for the new M3 streaming
//! process API (roadmap §5.B3–B6). The M3 implementation
//! uses `tokio::process::Command` under the hood so plugins
//! can build up a command, spawn it, and stream
//! stdin/stdout/stderr without blocking the plugin worker
//! thread.

pub mod access;
pub mod child;
pub mod command;
pub mod output;
pub mod stdio;
