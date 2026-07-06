//! M4-T2: `ui.Table` userdata — grid of Rows of Cells.

use super::cell::Cell;
use mlua::{MetaMethod, UserData, UserDataMethods};

/// A `ui.Row({Cell, ...})` userdata.
#[derive(Debug, Clone)]
pub struct Row {
    pub cells: Vec<Cell>,
}

impl Row {
    pub fn new(cells: Vec<Cell>) -> Self {
        Self { cells }
    }
}

impl UserData for Row {}

/// `ui.Row({...})` callable.
pub fn bind_row(lua: &mlua::Lua, parent: &mlua::Table<'_>) -> mlua::Result<()> {
    let row = lua.create_table()?;
    row.set(
        "__call",
        lua.create_function(|lua_ctx, args: mlua::MultiValue| {
            let mut iter = args.into_iter();
            let first = iter.next().unwrap_or(mlua::Value::Nil);
            let cells = match first {
                mlua::Value::Table(t) => {
                    let mut cs = Vec::new();
                    for i in 1..=t.len().unwrap_or(0) {
                        if let Ok(v) = t.get::<_, mlua::Value>(i) {
                            if let mlua::Value::UserData(ud) = v {
                                if let Ok(c) = ud.borrow::<Cell>() {
                                    cs.push(c.clone());
                                }
                            }
                        }
                    }
                    Row::new(cs)
                }
                _ => Row::new(Vec::new()),
            };
            lua_ctx.create_userdata(cells).map(mlua::Value::UserData)
        })?,
    )?;
    let mt = lua.create_table()?;
    mt.set("__metatable", mlua::Value::Boolean(false))?;
    row.set_metatable(Some(mt));
    parent.set("Row", row)
}

/// A `ui.Table({Row, ...})` userdata.
#[derive(Debug, Clone)]
pub struct Table {
    pub header: Option<Row>,
    pub rows: Vec<Row>,
    pub widths: Option<Vec<usize>>,
}

impl Table {
    pub fn new() -> Self {
        Self { header: None, rows: Vec::new(), widths: None }
    }
}

impl Default for Table { fn default() -> Self { Self::new() } }

impl UserData for Table {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method_mut("header", |_lua, this, row: mlua::AnyUserData| {
            let r = row.borrow::<Row>().map_err(|e| mlua::Error::RuntimeError(format!("{e}")))?;
            this.header = Some(r.clone());
            Ok(this.clone())
        });
        methods.add_method_mut("push", |_lua, this, row: mlua::AnyUserData| {
            let r = row.borrow::<Row>().map_err(|e| mlua::Error::RuntimeError(format!("{e}")))?;
            this.rows.push(r.clone());
            Ok(this.clone())
        });
        methods.add_meta_method(MetaMethod::ToString, |_lua, this, ()| {
            Ok(format!("Table(rows={})", this.rows.len()))
        });
    }
}

/// `ui.Table({Row, ...})` callable.
pub fn bind(lua: &mlua::Lua, parent: &mlua::Table<'_>) -> mlua::Result<()> {
    let table = lua.create_table()?;
    table.set(
        "__call",
        lua.create_function(|lua_ctx, args: mlua::MultiValue| {
            let mut iter = args.into_iter();
            let first = iter.next().unwrap_or(mlua::Value::Nil);
            let t = match first {
                mlua::Value::Table(tbl) => {
                    let mut rows = Vec::new();
                    for i in 1..=tbl.len().unwrap_or(0) {
                        if let Ok(v) = tbl.get::<_, mlua::Value>(i) {
                            if let mlua::Value::UserData(ud) = v {
                                if let Ok(r) = ud.borrow::<Row>() {
                                    rows.push(r.clone());
                                }
                            }
                        }
                    }
                    Table { header: None, rows, widths: None }
                }
                _ => Table::new(),
            };
            lua_ctx.create_userdata(t).map(mlua::Value::UserData)
        })?,
    )?;
    let mt = lua.create_table()?;
    let inner_call = table.get::<_, mlua::Function>("__call")?;
    mt.set("__call", inner_call)?;
    mt.set("__metatable", mlua::Value::Boolean(false))?;
    table.set_metatable(Some(mt));
    parent.set("Table", table)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_table_new() {
        let t = Table::new();
        assert!(t.rows.is_empty());
    }
}
