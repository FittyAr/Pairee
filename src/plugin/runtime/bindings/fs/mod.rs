//! `pairee.fs` — filesystem + process helpers for plugins.

mod extra;
mod jail;
mod ops;
mod path;
mod spawn;
mod target;

use crate::plugin::command_policy::CommandPolicy;
use crate::plugin::manager::PluginRequest;
pub use jail::FsPolicy;
use mlua::{Lua, Table};
use std::sync::Arc;
use tokio::sync::mpsc;

/// Bind `pairee.fs`. `policy` is captured by every closure so the sandbox
/// cannot be altered from Lua.
pub fn bind(
    lua: &Lua,
    policy: FsPolicy,
    commands: Arc<CommandPolicy>,
    tx: mpsc::Sender<PluginRequest>,
) -> mlua::Result<Table<'_>> {
    let policy = Arc::new(policy);
    let fs = lua.create_table()?;
    ops::bind_core(lua, &fs, &policy)?;
    extra::bind_extra(lua, &fs, &policy)?;
    spawn::bind_spawn(lua, &fs, &policy, commands, tx)?;
    Ok(fs)
}
