//! Composer: namespace entries that are built the first time a plugin
//! reads them (`pairee.image`, `pairee.rt`, `pairee.th`), so a
//! plugin that never touches them pays nothing for them. A built entry is
//! stored in the table (later reads are plain lookups); unknown names read
//! as `nil`. `pairs()` lists only the entries built so far.

use mlua::{Lua, Table, Value};
use std::collections::HashMap;

/// Builds one entry of a composed namespace.
pub type Builder = Box<dyn Fn(&Lua) -> mlua::Result<Value> + Send + Sync>;

/// Makes the missing entries of `table` (e.g. `pairee`) come from
/// `builders` on first access; the entries it already has are untouched.
pub fn install(
    lua: &Lua,
    table: &Table,
    builders: Vec<(&'static str, Builder)>,
) -> mlua::Result<()> {
    let builders: HashMap<&'static str, Builder> = builders.into_iter().collect();
    let meta = lua.create_table()?;
    meta.set(
        "__index",
        lua.create_function(move |lua, (table, key): (Table, Value)| {
            let Value::String(name) = &key else {
                return Ok(Value::Nil);
            };
            // A key that is not UTF-8 names no namespace (nil, no error).
            let Ok(name) = name.to_str() else {
                return Ok(Value::Nil);
            };
            let Some(build) = builders.get(name.as_ref()) else {
                return Ok(Value::Nil);
            };
            let value = build(lua)?;
            table.raw_set(key, value.clone())?;
            Ok(value)
        })?,
    )?;
    table.set_metatable(Some(meta))
}

/// A [`Builder`] from a closure.
pub fn builder(f: impl Fn(&Lua) -> mlua::Result<Value> + Send + Sync + 'static) -> Builder {
    Box::new(f)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn entries_are_built_once_on_first_read() {
        let lua = Lua::new();
        let calls = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&calls);
        let ns = lua.create_table().unwrap();
        ns.set("plain", 1).unwrap();
        install(
            &lua,
            &ns,
            vec![(
                "answer",
                builder(move |_| {
                    counter.fetch_add(1, Ordering::SeqCst);
                    Ok(Value::Integer(42))
                }),
            )],
        )
        .unwrap();
        lua.globals().set("ns", ns).unwrap();
        assert_eq!(lua.load("return ns.plain").eval::<i64>().unwrap(), 1);
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        let (a, b, missing): (i64, i64, Value) = lua
            .load("return ns.answer, ns.answer, ns.nothing")
            .eval()
            .unwrap();
        assert_eq!((a, b), (42, 42));
        assert!(missing.is_nil());
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        let odd: Value = lua.load(r#"return ns["\xff"]"#).eval().unwrap();
        assert!(odd.is_nil());
    }
}
