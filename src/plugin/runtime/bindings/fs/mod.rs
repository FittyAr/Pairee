//! `pairee.fs` — filesystem + process helpers for plugins.

mod extra;
mod ops;
mod path;
mod spawn;

use crate::plugin::manager::PluginRequest;
use mlua::{Lua, Table};
use tokio::sync::mpsc;

pub fn bind(lua: &Lua, trusted: bool, tx: mpsc::Sender<PluginRequest>) -> mlua::Result<Table<'_>> {
    let fs = lua.create_table()?;
    ops::bind_core(lua, &fs)?;
    extra::bind_extra(lua, &fs)?;
    spawn::bind_spawn(lua, &fs, trusted, tx)?;
    Ok(fs)
}
