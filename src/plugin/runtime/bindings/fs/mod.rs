//! `pairee.fs` — filesystem + process helpers for plugins.

mod extra;
pub(crate) mod jail;
mod ops;
mod path;
pub mod remote;
mod spawn;
mod target;

use crate::plugin::command_policy::CommandPolicy;
use crate::plugin::manager::PluginRequest;
pub use jail::{Access, FsPolicy, contain, normalize_lexical};
use mlua::{Lua, Table};
use std::sync::Arc;
pub use target::Target;
pub(crate) use target::blocking;
use tokio::sync::mpsc;

/// Bind `pairee.fs`. `policy` is captured by every closure so the sandbox
/// cannot be altered from Lua.
pub fn bind(
    lua: &Lua,
    policy: Arc<FsPolicy>,
    commands: Arc<CommandPolicy>,
    tx: mpsc::Sender<PluginRequest>,
) -> mlua::Result<Table> {
    let fs = lua.create_table()?;
    ops::bind_core(lua, &fs, &policy)?;
    extra::bind_extra(lua, &fs, &policy)?;
    let access = remote::RemoteAccess {
        mode: remote::RemoteMode::new(policy.trusted, policy.secure_mode),
        tx: tx.clone(),
    };
    remote::wrap(lua, &fs, access)?;
    spawn::bind_spawn(lua, &fs, &policy, commands, tx)?;
    Ok(fs)
}
