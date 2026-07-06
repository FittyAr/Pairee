//! M4-T2: `ui.List` userdata — a list of items.

use mlua::{MetaMethod, UserData, UserDataMethods};

/// A `ui.List({...})` userdata — a list of styled items.
#[derive(Debug, Clone)]
pub struct List {
    pub items: Vec<String>,
}

impl List {
    pub fn new(items: Vec<String>) -> Self {
        Self { items }
    }
}

impl UserData for List {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method_mut("push", |_lua, this, item: String| {
            this.items.push(item);
            Ok(this.clone())
        });
        methods.add_meta_method(MetaMethod::ToString, |_lua, this, ()| {
            Ok(format!("List(items={})", this.items.len()))
        });
    }
}

/// `ui.List({...})` callable.
pub fn bind(lua: &mlua::Lua, parent: &mlua::Table<'_>) -> mlua::Result<()> {
    let list = lua.create_table()?;
    list.set(
        "__call",
        lua.create_function(|lua_ctx, args: mlua::MultiValue| {
            let mut iter = args.into_iter();
            let first = iter.next().unwrap_or(mlua::Value::Nil);
            let item_list = match first {
                mlua::Value::Table(t) => {
                    let mut items = Vec::new();
                    for i in 1..=t.len().unwrap_or(0) {
                        if let Ok(v) = t.get::<_, mlua::String>(i) {
                            if let Ok(c) = v.to_str() { items.push(c.to_string()); }
                        }
                    }
                    List::new(items)
                }
                _ => List::new(Vec::new()),
            };
            lua_ctx.create_userdata(item_list).map(mlua::Value::UserData)
        })?,
    )?;
    let mt = lua.create_table()?;
    let inner_call = list.get::<_, mlua::Function>("__call")?;
    mt.set("__call", inner_call)?;
    mt.set("__metatable", mlua::Value::Boolean(false))?;
    list.set_metatable(Some(mt));
    parent.set("List", list)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_new_from_vec() {
        let items = vec!["a".to_string(), "b".to_string()];
        let l = List::new(items);
        assert_eq!(l.items.len(), 2);
    }
}
