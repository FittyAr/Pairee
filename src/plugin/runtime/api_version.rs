//! Semver of the public Lua surface (`pairee.*`).
//!
//! Bump independently of the Pairee crate version:
//! - **MAJOR** — remove or rename a stable binding
//! - **MINOR** — add a binding or field
//! - **PATCH** — docs / bugfix with the same signature

pub const LUA_API_VERSION: &str = "1.0.0";
