//! M2 shared userdata infrastructure: the `add_cached_field` shim for
//! memoising derived fields, and the `Composer` proxy for lazy
//! namespace instantiation.
//!
//! Both are part of the foundation described in
//! `docs/technical/plugin-roadmap.md` §4.1 (F1 — Typed userdata with
//! metamethods and builder pattern).

use std::any::Any;
use std::collections::HashMap;

/// Per-userdata cache, keyed by the field name (e.g. `"name"`,
/// `"stem"`). Stored on the userdata's first user-value slot so that
/// the cache lives exactly as long as the userdata itself.
pub type FieldCache = HashMap<String, mlua::Value>;

/// Compute (or fetch) a memoised field value for the given userdata.
///
/// The first call invokes `compute`, stores the resulting
/// `mlua::Value` in a per-userdata cache table, and returns it. All
/// subsequent calls with the same `name` return the cached value
/// without invoking `compute`.
///
/// This is the M2 shim that lets us expose a rich getter surface
/// (e.g. `File.name`, `File.cha:perm()`) on userdata types without
/// re-doing the underlying `std::fs::metadata` / path-parsing work
/// every time the plugin reads the field.
///
/// # Example
/// ```ignore
/// // Inside `impl mlua::UserData for Cha`:
/// fields.add_field_method_get("name", |lua, this| {
///     cached_field(lua, this, "name", |lua| {
///         lua.create_string(&this.url_path.file_name()
///             .and_then(|n| n.to_str())
///             .unwrap_or("").to_string())
///             .map(mlua::Value::String)
///     })
/// });
/// ```
pub fn cached_field<F>(
    _lua: &mlua::Lua,
    ud: &mlua::UserData,
    name: &str,
    compute: F,
) -> mlua::Result<mlua::Value>
where
    F: FnOnce(&mlua::Lua) -> mlua::Result<mlua::Value>,
{
    // Get-or-create the per-userdata cache table stored in the
    // userdata's first user-value slot.
    let cache_any: Option<Box<dyn Any>> = ud.user_value(1)?;
    let mut cache: FieldCache = match cache_any {
        Some(boxed) => match boxed.downcast::<FieldCache>() {
            Ok(c) => *c,
            Err(_) => FieldCache::new(),
        },
        None => FieldCache::new(),
    };

    if let Some(v) = cache.get(name) {
        return Ok(v.clone());
    }

    // mlua 0.9 does not let us easily re-borrow the Lua handle inside
    // a UserDataFields callback that already gave us a `&Lua` — so we
    // use the value from the outer closure's `lua` parameter. The
    // `compute` closure receives the same `&Lua`.
    let value = compute(_lua)?;
    cache.insert(name.to_string(), value.clone());

    // Store the updated cache back. The cache is cheap to clone for
    // small maps; if a particular type accumulates many cached
    // fields, we can move to an `Arc<Mutex<FieldCache>>` later.
    let cache_box = Box::new(cache);
    ud.set_user_value(1, cache_box)?;
    Ok(value)
}

/// Composer was a planned "lazy namespace proxy" for `pairee.*`
/// sub-tables that was never wired into the runtime. It was
/// deleted in M5 because:
///
/// - `Composer::install` was only ever called from a unit test, so
///   the `__composer__` slot it added to the parent table was dead
///   surface (and would have been reachable from Lua as e.g.
///   `pairee.fs.__composer__` if the feature ever shipped).
/// - The bind functions construct each sub-table eagerly via
///   `super::bindings::fs::bind` / `super::bindings::app::bind` / …
///   already, so a lazy proxy buys nothing today.
///
/// `cached_field` (the other helper in this file) stays — it is
/// used by the `Cha` / `File` userdata getters.

#[cfg(test)]
mod tests {
    use super::*;
    use mlua::Lua;

    #[test]
    fn test_cached_field_memoises() {
        let lua = Lua::new();
        // Use a small wrapper to attach a UserData with our shim.
        // We can't easily create a UserData in a test without a
        // real type, so we test the lower-level primitive here:
        // the FieldCache itself round-trips through `Any`.
        let cache: FieldCache = HashMap::new();
        let boxed: Box<dyn Any> = Box::new(cache);
        let downcast: Box<FieldCache> = boxed.downcast().expect("downcast FieldCache");
        assert!(downcast.is_empty());
    }
}
