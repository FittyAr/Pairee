//! Legacy `pairee.fs.spawn` and `spawn_copy_task`.

use super::path::{Access, FsPolicy, validate_path};
use crate::plugin::command_policy::CommandPolicy;
use crate::plugin::manager::PluginRequest;
use crate::plugin::runtime::bindings::process::{LuaCommand, StdioKind};
use mlua::{Lua, Table};
use std::sync::Arc;
use tokio::sync::mpsc;

pub fn bind_spawn(
    lua: &Lua,
    fs: &Table,
    policy: &Arc<FsPolicy>,
    commands: Arc<CommandPolicy>,
    tx: mpsc::Sender<PluginRequest>,
) -> mlua::Result<()> {
    // The legacy form of `pairee.Command(cmd):arg(args):output()`, with
    // the same policy checks (no input from the terminal).
    let sandbox = Arc::clone(policy);
    fs.set(
        "spawn",
        lua.create_async_function(move |lua_ctx, (cmd, args): (String, Vec<String>)| {
            let mut command = LuaCommand::new(cmd, Arc::clone(&commands), Arc::clone(&sandbox));
            command.args = args;
            command.stdin = StdioKind::Null;
            async move {
                let out = command.output_inner().await?;
                let t = lua_ctx.create_table()?;
                t.set("stdout", out.stdout)?;
                t.set("stderr", out.stderr)?;
                t.set("status", out.status.code.unwrap_or(0))?;
                Ok(t)
            }
        })?,
    )?;

    let tx_copy = tx;
    let p = Arc::clone(policy);
    fs.set(
        "spawn_copy_task",
        lua.create_async_function(move |_, (from_str, to_str): (String, String)| {
            let tx = tx_copy.clone();
            let p = Arc::clone(&p);
            async move {
                // The transfer engine works on plain paths: the jail check
                // is check-then-use here (see THREAT_MODEL residual risk).
                let from = validate_path(&p, &from_str, Access::Read)?
                    .path()
                    .to_path_buf();
                let to = validate_path(&p, &to_str, Access::Write)?
                    .path()
                    .to_path_buf();
                let _ = tx.send(PluginRequest::SpawnCopyTask { from, to }).await;
                Ok(())
            }
        })?,
    )?;

    Ok(())
}
