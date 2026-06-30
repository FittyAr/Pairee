use crate::plugin::manager::PluginRequest;
use std::path::PathBuf;
use tokio::sync::mpsc;

fn is_secure_mode(lua: &mlua::Lua) -> bool {
    if let Ok(pairee) = lua.globals().get::<_, mlua::Table>("pairee") {
        pairee.get::<_, bool>("_secure_mode").unwrap_or(false)
    } else {
        false
    }
}

fn validate_path(lua: &mlua::Lua, path_str: &str) -> mlua::Result<PathBuf> {
    let path = PathBuf::from(path_str);
    if is_secure_mode(lua) {
        let abs_path = std::fs::canonicalize(&path).unwrap_or_else(|_| path.clone());
        let workspace = std::env::current_dir().unwrap_or_default();
        let config = crate::config::paths::get_config_dir();
        let cache = crate::config::paths::get_cache_dir();

        let in_workspace = abs_path.starts_with(&workspace);
        let in_config = abs_path.starts_with(&config);
        let in_cache = abs_path.starts_with(&cache);

        if !in_workspace && !in_config && !in_cache {
            return Err(mlua::Error::RuntimeError(format!(
                "Security violation: path {:?} is outside permitted sandboxed directories in Secure Mode",
                path
            )));
        }
    }
    Ok(path)
}

pub fn bind(
    lua: &mlua::Lua,
    trusted: bool,
    tx: mpsc::Sender<PluginRequest>,
) -> mlua::Result<mlua::Table<'_>> {
    let fs = lua.create_table()?;

    // read(path)
    fs.set(
        "read",
        lua.create_function(move |lua_ctx, path_str: String| {
            let path = validate_path(lua_ctx, &path_str)?;
            std::fs::read_to_string(&path)
                .map_err(|e| mlua::Error::RuntimeError(format!("Failed to read file: {}", e)))
        })?,
    )?;

    // write(path, data)
    fs.set(
        "write",
        lua.create_function(move |lua_ctx, (path_str, data): (String, String)| {
            let path = validate_path(lua_ctx, &path_str)?;
            std::fs::write(&path, data)
                .map_err(|e| mlua::Error::RuntimeError(format!("Failed to write file: {}", e)))
        })?,
    )?;

    // exists(path)
    fs.set(
        "exists",
        lua.create_function(move |lua_ctx, path_str: String| {
            let path = validate_path(lua_ctx, &path_str)?;
            Ok(path.exists())
        })?,
    )?;

    // stat(path)
    fs.set(
        "stat",
        lua.create_function(move |lua_ctx, path_str: String| {
            let path = validate_path(lua_ctx, &path_str)?;
            if !path.exists() {
                return Ok(mlua::Value::Nil);
            }
            let is_dir = path.is_dir();
            let is_symlink = path.is_symlink();
            let size = path.metadata().map(|m| m.len()).unwrap_or(0);
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();

            let t = lua_ctx.create_table()?;
            t.set("name", name)?;
            t.set("url", path_str.clone())?;
            t.set("path", path_str)?;
            t.set("size", size)?;
            t.set("is_dir", is_dir)?;
            t.set("is_symlink", is_symlink)?;
            Ok(mlua::Value::Table(t))
        })?,
    )?;

    // list(path)
    fs.set(
        "list",
        lua.create_function(move |lua_ctx, path_str: String| {
            let path = validate_path(lua_ctx, &path_str)?;
            let mut entries_vec = Vec::new();
            if let Ok(entries) = std::fs::read_dir(&path) {
                for entry in entries.flatten() {
                    let p = entry.path();
                    let is_dir = p.is_dir();
                    let is_symlink = p.is_symlink();
                    let size = p.metadata().map(|m| m.len()).unwrap_or(0);
                    let name = p
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default();

                    let t = lua_ctx.create_table()?;
                    t.set("name", name)?;
                    t.set("url", p.to_string_lossy().to_string())?;
                    t.set("path", p.to_string_lossy().to_string())?;
                    t.set("size", size)?;
                    t.set("is_dir", is_dir)?;
                    t.set("is_symlink", is_symlink)?;
                    entries_vec.push(t);
                }
            }
            Ok(entries_vec)
        })?,
    )?;

    // spawn(cmd, args)
    fs.set("spawn", lua.create_async_function(move |lua_ctx, (cmd, args): (String, Vec<String>)| {
        async move {
            if !trusted {
                return Err(mlua::Error::RuntimeError(
                    "Security violation: spawning external processes is blocked in sandboxed mode.".to_string()
                ));
            }
            if is_secure_mode(lua_ctx) && !crate::plugin::sandbox::is_command_safe(&cmd) {
                return Err(mlua::Error::RuntimeError(format!(
                    "Security violation: Command '{}' is blacklisted in Secure Mode",
                    cmd
                )));
            }

            // Execute process
            let output = tokio::process::Command::new(&cmd)
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
                Err(e) => Err(mlua::Error::RuntimeError(format!("Failed to spawn process: {}", e))),
            }
        }
    })?)?;

    // spawn_copy_task(from, to)
    let tx_copy = tx.clone();
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

    Ok(fs)
}
