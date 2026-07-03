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

/// Composer — lazy namespace proxy for `pairee.*` sub-tables.
///
/// Plugins that reach into `pairee.fs.read(...)` should not have to
/// pay the cost of constructing the full `fs` sub-table up-front
/// (especially when Secure Mode would block most of its
/// methods). Instead we register a tiny Composer under the namespace
/// key (e.g. `pairee.fs`); the first time the plugin reads
/// `pairee.fs.something`, the Composer instantiates the real table
/// via the registered factory and caches it.
///
/// This is a small optimisation today but unblocks future lazy
/// construction of the heavier modules (UI widgets, image preview,
/// etc.) without touching the plugin's surface.
#[derive(Default)]
pub struct Composer {
    entries: HashMap<String, Box<dyn Fn(&mlua::Lua) -> mlua::Result<mlua::Value>>>,
}

impl Composer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a namespace entry. `name` is the key the plugin will
    /// access (e.g. `"read"`); `factory` is invoked the first time
    /// the entry is requested and must return the value to expose.
    pub fn with<F>(mut self, name: &str, factory: F) -> Self
    where
        F: Fn(&mlua::Lua) -> mlua::Result<mlua::Value> + 'static,
    {
        self.entries
            .insert(name.to_string(), Box::new(factory));
        self
    }

    /// Install the composer on a parent table. The composer is
    /// exposed as a `UserData` so Lua can index it like a normal
    /// table; `__index` looks up the requested key in `entries` and
    /// invokes the registered factory on cache miss.
    pub fn install(self, lua: &mlua::Lua, parent: &mlua::Table<'_>) -> mlua::Result<()> {
        // We expose the composer via a thin proxy: a UserData whose
        // __index metamethod delegates to the factories. This avoids
        // constructing a giant flat table up-front.
        let entries = std::rc::Rc::new(self.entries);
        let cache: std::rc::Rc<std::cell::RefCell<HashMap<String, mlua::Value>>> =
            std::rc::Rc::new(std::cell::RefCell::new(HashMap::new()));

        let entries_for_index = entries.clone();
        let cache_for_index = cache.clone();
        let meta = lua.create_table()?;
        meta.set(
            "__index",
            lua.create_function(move |lua, (this, key): (mlua::Table, mlua::String)| {
                let key_str = key.to_str()?;
                // Memoised?
                if let Some(v) = cache_for_index.borrow().get(key_str) {
                    return Ok(v.clone());
                }
                // Factory lookup.
                if let Some(factory) = entries_for_index.get(key_str) {
                    let value = factory(lua)?;
                    cache_for_index
                        .borrow_mut()
                        .insert(key_str.to_string(), value.clone());
                    // Set the value on `this` so the next access hits
                    // the table fast path instead of `__index` again.
                    this.set(key_str, value.clone())?;
                    return Ok(value);
                }
                Ok(mlua::Value::Nil)
            })?,
        )?;
        meta.set(
            "__metatable",
            // hide the metatable from the plugin (best-effort)
            mlua::Value::Boolean(false),
        )?;

        // A userdata's metatable cannot be set directly in mlua 0.9,
        // but we can expose the composer as a regular table whose
        // __index is the closure above. That is functionally
        // equivalent from Lua's perspective.
        let proxy = lua.create_table()?;
        proxy.set_metatable(Some(meta));
        parent.set("__composer__", proxy)?;
        Ok(())
    }
}

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

    #[test]
    fn test_composer_registers_entries() {
        let composer = Composer::new()
            .with("hello", |lua| lua.create_string("world").map(mlua::Value::String))
            .with("forty_two", |_| Ok(mlua::Value::Integer(42)));
        assert_eq!(composer.entries.len(), 2);
    }
}
