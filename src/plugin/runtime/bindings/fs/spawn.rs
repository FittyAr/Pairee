//! Legacy `pairee.fs.spawn` and `spawn_copy_task`.

use super::path::{Access, FsPolicy, validate_path};
use crate::plugin::command_policy::CommandPolicy;
use crate::plugin::manager::PluginRequest;
use mlua::{Lua, Table};
use std::sync::Arc;
use tokio::sync::mpsc;

pub fn bind_spawn(
    lua: &Lua,
    fs: &Table<'_>,
    policy: &Arc<FsPolicy>,
    commands: Arc<CommandPolicy>,
    tx: mpsc::Sender<PluginRequest>,
) -> mlua::Result<()> {
    fs.set(
        "spawn",
        lua.create_async_function(move |lua_ctx, (cmd, args): (String, Vec<String>)| {
            let commands = Arc::clone(&commands);
            async move {
                let program = commands
                    .authorize(&cmd)
                    .map_err(mlua::Error::RuntimeError)?;
                let output = tokio::process::Command::new(&program)
                    .args(&args)
                    .output()
                    .await;
                match output {
                    Ok(out) => {
                        let t = lua_ctx.create_table()?;
                        t.set("stdout", String::from_utf8_lossy(&out.stdout).to_string())?;
                        t.set("stderr", String::from_utf8_lossy(&out.stderr).to_string())?;
                        t.set("status", out.status.code().unwrap_or(0))?;
                        Ok(t)
                    }
                    Err(e) => Err(mlua::Error::RuntimeError(format!(
                        "Failed to spawn process: {e}"
                    ))),
                }
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
                let from = validate_path(&p, &from_str, Access::Read)?;
                let to = validate_path(&p, &to_str, Access::Write)?;
                let _ = tx.send(PluginRequest::SpawnCopyTask { from, to }).await;
                Ok(())
            }
        })?,
    )?;

    Ok(())
}
