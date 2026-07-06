//! M4-T2: `ui.Cell` userdata — a single cell in a table row.

use super::span::Span;
use mlua::UserData;

/// A `ui.Cell(...)` userdata — wraps a single Span or raw string.
#[derive(Debug, Clone)]
pub struct Cell {
    pub content: Span,
}

impl Cell {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            content: Span::new(text),
        }
    }

    pub fn from_span(span: Span) -> Self {
        Self { content: span }
    }
}

impl UserData for Cell {
    fn add_methods<'lua, M: mlua::UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method("text", |_lua, this, ()| Ok(this.content.text.clone()));
    }
}

/// `ui.Cell(value)` callable.
pub fn bind(lua: &mlua::Lua, parent: &mlua::Table<'_>) -> mlua::Result<()> {
    let cell = lua.create_table()?;
    cell.set(
        "__call",
        lua.create_function(|lua_ctx, args: mlua::MultiValue| {
            let mut iter = args.into_iter();
            let first = iter.next().unwrap_or(mlua::Value::Nil);
            let c = match first {
                mlua::Value::String(s) => Cell::new(s.to_str().map(|c| c.to_string()).unwrap_or_default()),
                mlua::Value::UserData(ud) if ud.borrow::<Span>().is_ok() => Cell::from_span(ud.borrow::<Span>().ok().unwrap().clone()),
                _ => Cell::new(""),
            };
            lua_ctx.create_userdata(c).map(mlua::Value::UserData)
        })?,
    )?;
    let mt = lua.create_table()?;
    let inner_call = cell.get::<_, mlua::Function>("__call")?;
    mt.set("__call", inner_call)?;
    mt.set("__metatable", mlua::Value::Boolean(false))?;
    cell.set_metatable(Some(mt));
    parent.set("Cell", cell)
}
