//! Legacy `pairee.fs.spawn` and `spawn_copy_task`.

use super::path::{is_secure_mode, validate_path};
use crate::plugin::manager::PluginRequest;
use mlua::{Lua, Table};
use tokio::sync::mpsc;

pub fn bind_spawn(
    lua: &Lua,
    fs: &Table<'_>,
    trusted: bool,
    tx: mpsc::Sender<PluginRequest>,
) -> mlua::Result<()> {
    fs.set(
        "spawn",
        lua.create_async_function(move |lua_ctx, (cmd, args): (String, Vec<String>)| {
            async move {
                if !trusted {
                    return Err(mlua::Error::RuntimeError(
                        "Security violation: spawning external processes is blocked in sandboxed mode."
                            .to_string(),
                    ));
                }
                if is_secure_mode(lua_ctx) && !crate::plugin::sandbox::is_command_safe(&cmd) {
                    return Err(mlua::Error::RuntimeError(format!(
                        "Security violation: Command '{cmd}' is blacklisted in Secure Mode"
                    )));
                }

                let output = tokio::process::Command::new(&cmd).args(&args).output().await;
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
    fs.set(
        "spawn_copy_task",
        lua.create_async_function(move |lua_ctx, (from_str, to_str): (String, String)| {
            let tx = tx_copy.clone();
            async move {
                let from = validate_path(lua_ctx, &from_str)?;
                let to = validate_path(lua_ctx, &to_str)?;
                let _ = tx.send(PluginRequest::SpawnCopyTask { from, to }).await;
                Ok(())
            }
        })?,
    )?;

    Ok(())
}
