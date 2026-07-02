//! Basic utility bindings for plugins (M0).
//!
//! Exposes the cross-platform `target_os` / `target_family` strings, the
//! UNIX epoch time, and a fast non-cryptographic string hash (xxhash-style
//! via the standard library's `DefaultHasher` for portability — a faster
//! algorithm can be substituted later without changing the API).
//!
//! All functions in this module are synchronous and side-effect-free.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn bind(lua: &mlua::Lua) -> mlua::Result<mlua::Table<'_>> {
    let utils = lua.create_table()?;

    // `pairee.target_os()` — returns the compile-time OS string.
    utils.set(
        "target_os",
        lua.create_function(|_, ()| Ok(std::env::consts::OS))?,
    )?;

    // `pairee.target_family()` — returns the compile-time OS family
    // ("unix" | "windows" | "wasm").
    utils.set(
        "target_family",
        lua.create_function(|_, ()| Ok(std::env::consts::FAMILY))?,
    )?;

    // `pairee.time()` — returns the current UNIX epoch in seconds as a
    // floating-point number, or `nil` if the system clock is before the
    // epoch (which should not happen on any modern platform but is handled
    // for safety).
    utils.set(
        "time",
        lua.create_function(|_, ()| {
            let secs: mlua::Value = match SystemTime::now().duration_since(UNIX_EPOCH) {
                Ok(d) => mlua::Value::Number(d.as_secs_f64()),
                Err(_) => mlua::Value::Nil,
            };
            Ok(secs)
        })?,
    )?;

    // `pairee.hash(str)` — 64-bit non-cryptographic hash of a string,
    // returned as a hex-encoded lowercase string.
    utils.set(
        "hash",
        lua.create_function(|_lua, s: mlua::String| {
            let mut hasher = DefaultHasher::new();
            s.as_bytes().hash(&mut hasher);
            Ok(format!("{:016x}", hasher.finish()))
        })?,
    )?;

    Ok(utils)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_target_os_is_nonempty() {
        assert!(!std::env::consts::OS.is_empty());
    }

    #[test]
    fn test_target_family_is_recognised() {
        let f = std::env::consts::FAMILY;
        assert!(matches!(f, "unix" | "windows" | "wasm"));
    }

    #[test]
    fn test_hash_is_stable_for_same_input() {
        let mut h1 = DefaultHasher::new();
        let mut h2 = DefaultHasher::new();
        "hello".hash(&mut h1);
        "hello".hash(&mut h2);
        assert_eq!(h1.finish(), h2.finish());
    }

    #[test]
    fn test_hash_differs_for_different_inputs() {
        let mut h1 = DefaultHasher::new();
        let mut h2 = DefaultHasher::new();
        "hello".hash(&mut h1);
        "world".hash(&mut h2);
        assert_ne!(h1.finish(), h2.finish());
    }

    #[test]
    fn test_time_is_positive_after_2020() {
        // After 2020-01-01, the UNIX epoch in seconds is > 1.5e9.
        let secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock is past epoch")
            .as_secs_f64();
        assert!(secs > 1_577_836_800.0);
    }
}
