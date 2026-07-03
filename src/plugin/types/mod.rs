//! M2 typed userdata surface.
//!
//! This module re-exports every type that plugins can see in the new
//! typed-userdata world:
//!
//! - `Url` / `PathU` (in `url`) — path-or-URI wrappers.
//! - `Cha` (in `cha`) — file characteristics.
//! - `File` (in `file`) — the main entry point, derefs to `Cha`.
//! - `Error` (in `error`) — the standard error envelope.
//!
//! The constructor registry that exposes them to Lua (e.g. `Url("…")`,
//! `Cha{…}`, `File{url=…}`) lives in `runtime::types::register`.

pub mod cha;
pub mod error;
pub mod file;
pub mod url;

pub use cha::{Cha, ChaKind, ChaMode};
pub use error::Error;
pub use file::File;
pub use url::{PathU, Scheme, Url};
