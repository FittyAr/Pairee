//! Lua binding for the runtime keybindings API.
//!
//! Plugins can ask the host to bind a key to one of their
//! callbacks at runtime through
//! `pairee.keybindings.bind(key, action, opts?)`. The host runs
//! the registration through the same `KeybindingResolver` that
//! the manifest's `[keybindings]` table feeds, so the priority
//! table and the conflict-detection rules are identical.
//!
//! ## Example
//!
//! ```lua
//! -- "steal" mode: take the key even if it is owned by a
//! -- Builtin or another Plugin binding. Power-user only.
//! local ok, err = pairee.keybindings.bind("F2", "entry", { mode = "steal" })
//! if not ok then
//!     pairee.log.warn("could not bind F2: " .. tostring(err))
//! end
//! ```
//!
//! ## Modes
//!
//! | `mode`             | Behaviour                                                |
//! |--------------------|----------------------------------------------------------|
//! | `"fallback"` (default) | The binding only lands if the key is free.          |
//! | `"steal"`          | Override the current owner (any source).                |
//! | `"fail"`           | Strict: error out unless the key is currently unbound.   |
//!
//! `"steal"` is gated behind a `ConflictPolicy::Override` that
//! the resolver applies internally; the plugin just sees a
//! boolean success and (on failure) a human-readable error.

use crate::keybindings::resolver::KeybindingResolver;
use crate::keybindings::source::{ConflictPolicy, RegisterOutcome, ResolvedBinding};
use mlua::{Lua, Table, Value};

/// Build the `pairee.keybindings` Lua module.
///
/// `plugin_name` is the manifest's `name` field; it travels
/// with every binding so the dispatcher can route the keypress
/// to the right plugin later. `resolver` is the live resolver
/// that the host is using for the current frame; bindings
/// installed through this module are visible immediately to the
/// main loop on the next keypress.
///
/// The `plugin_name` and `resolver` are `Arc`-wrapped internally
/// so the Lua closures can live as long as the Lua VM and the
/// resolver does (which outlives any single plugin load).
pub fn build_bind_table(
    lua: &Lua,
    resolver: std::sync::Arc<KeybindingResolver>,
    plugin_name: std::sync::Arc<String>,
) -> mlua::Result<Table<'_>> {
    let table = lua.create_table()?;

    // `pairee.keybindings.bind(key, action, opts?) -> (ok, err_or_nil)`
    //
    // Returns a `(boolean, string?)` tuple. The boolean is
    // `true` on a successful binding and `false` on a conflict
    // or invalid input; the second value is a human-readable
    // error string (or `nil` on success).
    let resolver_for_bind = std::sync::Arc::clone(&resolver);
    let plugin_for_bind = std::sync::Arc::clone(&plugin_name);
    let bind = lua.create_function(
        move |lua, (key, action, opts): (String, String, Option<Table>)| {
            // Normalise every error path into a `(false, String)`
            // so Lua can distinguish success (`true, nil`) from
            // failure (`false, "reason"`).
            let to_lua_err = |s: String| -> mlua::Result<(bool, Value)> {
                Ok((false, Value::String(lua.create_string(&s)?)))
            };

            let mode = match parse_mode(opts.as_ref()) {
                Ok(m) => m,
                Err(e) => return to_lua_err(e),
            };
            let policy = match mode {
                Mode::Fallback => ConflictPolicy::Fallback,
                Mode::Steal => ConflictPolicy::Override,
                Mode::Fail => ConflictPolicy::Fallback,
            };

            let outcome = resolver_for_bind.register(
                &key,
                ResolvedBinding::lua(plugin_for_bind.as_str(), &action),
                policy,
            );

            match outcome {
                RegisterOutcome::Bound => Ok((true, Value::Nil)),
                RegisterOutcome::Conflict { with } => to_lua_err(format!(
                    "key '{}' is already bound by {}; use mode = 'steal' to override",
                    key, with
                )),
                RegisterOutcome::Invalid => {
                    to_lua_err(format!("key '{}' is invalid (empty or malformed)", key))
                }
            }
        },
    )?;

    table.set("bind", bind)?;

    // `pairee.keybindings.list() -> { {key, action, source}, ... }`
    //
    // Returns a snapshot of every binding currently in the
    // resolver. Useful for a plugin that wants to pick an
    // unused key at runtime, or for diagnostics.
    let resolver_for_list = std::sync::Arc::clone(&resolver);
    let list = lua.create_function(move |lua, ()| {
        let bindings = resolver_for_list.bindings();
        let out = lua.create_table()?;
        for (i, (key, binding)) in bindings.into_iter().enumerate() {
            let row = lua.create_table()?;
            row.set("key", key)?;
            row.set("plugin_action", binding.plugin_action)?;
            row.set("source", binding.source.label())?;
            out.set(i + 1, row)?;
        }
        Ok(out)
    })?;
    table.set("list", list)?;

    // `pairee.keybindings.unbind(key) -> (ok, err_or_nil)`
    //
    // Per-key unbind is not yet implemented; the function
    // returns an error so the plugin can branch. The bulk
    // `unbind_plugin` below covers the common "release
    // everything I own" case.
    let unbind = lua.create_function(|lua, _key: String| {
        Ok((
            false,
            Value::String(lua.create_string(
                "per-key unbind is not implemented yet; use unbind_plugin() to release \
                 every key this plugin owns",
            )?),
        ))
    })?;
    table.set("unbind", unbind)?;

    // `pairee.keybindings.unbind_plugin() -> count`
    //
    // Releases every binding the calling plugin owns (manifest
    // entries and runtime binds). Returns the number released.
    let resolver_for_unbind = std::sync::Arc::clone(&resolver);
    let plugin_for_unbind = std::sync::Arc::clone(&plugin_name);
    let unbind_plugin = lua.create_function(move |_lua, ()| -> mlua::Result<usize> {
        let name = plugin_for_unbind.as_str();
        let before = resolver_for_unbind.len();
        resolver_for_unbind.unregister_plugin(name);
        // `len` is the union of static + overlay; we want the
        // overlay-only delta. Subtract the static size to get
        // it. The arithmetic is exact (both halves are
        // non-negative) so an `as` cast is safe.
        let after = resolver_for_unbind.len();
        let released = before.saturating_sub(after);
        Ok(released)
    })?;
    table.set("unbind_plugin", unbind_plugin)?;

    Ok(table)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Fallback,
    Steal,
    Fail,
}

fn parse_mode(opts: Option<&Table>) -> Result<Mode, String> {
    let Some(opts) = opts else {
        return Ok(Mode::Fallback);
    };
    let mode: String = match opts.get::<_, Value>("mode") {
        Ok(Value::String(s)) => s.to_str().map_err(|e| e.to_string())?.to_string(),
        Ok(Value::Nil) | Err(_) => return Ok(Mode::Fallback),
        Ok(other) => {
            return Err(format!(
                "pairee.keybindings.bind: `mode` must be a string, got {:?}",
                other
            ));
        }
    };
    match mode.as_str() {
        "fallback" | "" => Ok(Mode::Fallback),
        "steal" | "override" => Ok(Mode::Steal),
        "fail" | "fail_if_occupied" => Ok(Mode::Fail),
        other => Err(format!(
            "pairee.keybindings.bind: unknown mode '{}' (expected 'fallback' | 'steal' | 'fail')",
            other
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The mode parser must accept the documented spellings
    /// (and the empty default). If a future refactor drops
    /// one, this test fails before the API regresses.
    #[test]
    fn parse_mode_accepts_documented_spellings() {
        let lua = Lua::new();
        // `nil` opts -> Fallback
        let opts: Option<Table> = None;
        assert_eq!(parse_mode(opts.as_ref()).unwrap(), Mode::Fallback);

        // Empty-string mode -> Fallback (some callers set
        // `mode = ""` as a sentinel).
        let t = lua.create_table().unwrap();
        t.set("mode", "").unwrap();
        assert_eq!(parse_mode(Some(&t)).unwrap(), Mode::Fallback);

        // `mode = "fallback"`.
        let t = lua.create_table().unwrap();
        t.set("mode", "fallback").unwrap();
        assert_eq!(parse_mode(Some(&t)).unwrap(), Mode::Fallback);

        // `mode = "steal"` and the alias `"override"`.
        let t = lua.create_table().unwrap();
        t.set("mode", "steal").unwrap();
        assert_eq!(parse_mode(Some(&t)).unwrap(), Mode::Steal);
        let t = lua.create_table().unwrap();
        t.set("mode", "override").unwrap();
        assert_eq!(parse_mode(Some(&t)).unwrap(), Mode::Steal);

        // `mode = "fail"` and the long form `"fail_if_occupied"`.
        let t = lua.create_table().unwrap();
        t.set("mode", "fail").unwrap();
        assert_eq!(parse_mode(Some(&t)).unwrap(), Mode::Fail);
        let t = lua.create_table().unwrap();
        t.set("mode", "fail_if_occupied").unwrap();
        assert_eq!(parse_mode(Some(&t)).unwrap(), Mode::Fail);
    }

    #[test]
    fn parse_mode_rejects_unknown_string() {
        let lua = Lua::new();
        let t = lua.create_table().unwrap();
        t.set("mode", "nope").unwrap();
        let err = parse_mode(Some(&t)).unwrap_err();
        assert!(err.contains("unknown mode"));
    }

    #[test]
    fn parse_mode_rejects_non_string_mode() {
        let lua = Lua::new();
        let t = lua.create_table().unwrap();
        t.set("mode", 42).unwrap();
        let err = parse_mode(Some(&t)).unwrap_err();
        assert!(err.contains("must be a string"));
    }
}
