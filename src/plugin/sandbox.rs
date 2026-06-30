use crate::plugin::manager::PluginRequest;
use std::path::Path;
use tokio::sync::mpsc;

pub fn is_command_safe(cmd: &str) -> bool {
    let path = Path::new(cmd);
    let bin_name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_lowercase())
        .unwrap_or_else(|| cmd.to_lowercase());

    let blacklist = [
        "curl",
        "wget",
        "nc",
        "netcat",
        "ssh",
        "scp",
        "sftp",
        "telnet",
        "ftp",
        "rsync",
        "nmap",
        "sh",
        "bash",
        "zsh",
        "csh",
        "tcsh",
        "powershell",
        "pwsh",
        "cmd",
        "cmd.exe",
        "python",
        "python3",
        "perl",
        "ruby",
        "node",
        "php",
        "lua",
        "luajit",
    ];

    !blacklist.contains(&bin_name.as_str())
}

pub fn create_sandboxed_lua(
    plugin_dir: &Path,
    trusted: bool,
    tx: mpsc::Sender<PluginRequest>,
) -> anyhow::Result<mlua::Lua> {
    // 1. Determine standard libraries to load
    let std_libs = if trusted {
        mlua::StdLib::ALL
    } else {
        mlua::StdLib::TABLE | mlua::StdLib::STRING | mlua::StdLib::UTF8 | mlua::StdLib::MATH
    };

    let lua = mlua::Lua::new_with(std_libs, mlua::LuaOptions::default())?;

    // 2. Untrusted sandboxing restrictions
    if !trusted {
        let globals = lua.globals();
        // Remove dangerous global evaluation/loading functions
        let _: Result<(), mlua::Error> = globals.set("load", mlua::Value::Nil);
        let _: Result<(), mlua::Error> = globals.set("loadstring", mlua::Value::Nil);
        let _: Result<(), mlua::Error> = globals.set("dofile", mlua::Value::Nil);
        let _: Result<(), mlua::Error> = globals.set("loadfile", mlua::Value::Nil);
    }

    // 3. Setup relative require wrapper
    setup_require_wrapper(&lua, plugin_dir)?;

    // 4. Bind the pairee global namespace
    crate::plugin::runtime::standard::bind_runtime(&lua, plugin_dir, trusted, tx)?;

    Ok(lua)
}

fn setup_require_wrapper(lua: &mlua::Lua, plugin_dir: &Path) -> mlua::Result<()> {
    let globals = lua.globals();
    let plugin_dir_clone = plugin_dir.to_path_buf();

    // Create a custom loader function
    let require_fn = lua.create_function(
        move |lua_ctx, module_name: String| -> mlua::Result<mlua::Value> {
            let globals = lua_ctx.globals();

            // Check package.loaded first
            let package: mlua::Table = globals.get("package")?;
            let loaded: mlua::Table = package.get("loaded")?;
            if loaded.contains_key(module_name.as_str())? {
                return loaded.get(module_name.as_str());
            }

            // Convert module name dot separator to directory separators
            let rel_path_str = module_name.replace('.', "/");
            let candidate_path = plugin_dir_clone.join(format!("{}.lua", rel_path_str));

            // Enforce sandbox path boundary (no directory traversal out of plugin dir)
            let canon_plugin = plugin_dir_clone.canonicalize().map_err(|e| {
                mlua::Error::RuntimeError(format!("Failed to canonicalize plugin path: {}", e))
            })?;

            let canon_candidate = match candidate_path.canonicalize() {
                Ok(c) => c,
                Err(e) => {
                    return Err(mlua::Error::RuntimeError(format!(
                        "Module {} not found or inaccessible: {}",
                        module_name, e
                    )));
                }
            };

            if !canon_candidate.starts_with(&canon_plugin) {
                return Err(mlua::Error::RuntimeError(format!(
                    "Security violation: module {} is outside the plugin root",
                    module_name
                )));
            }

            let code = std::fs::read_to_string(&canon_candidate).map_err(|e| {
                mlua::Error::RuntimeError(format!("Failed to read module file: {}", e))
            })?;

            // Load and execute module code
            let module_chunk = lua_ctx.load(&code);
            let module_val: mlua::Value = module_chunk.eval()?;

            // Cache in package.loaded
            loaded.set(module_name, module_val.clone())?;

            Ok(module_val)
        },
    )?;

    globals.set("require", require_fn)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_is_command_safe() {
        assert!(is_command_safe("cargo"));
        assert!(is_command_safe("git"));
        assert!(!is_command_safe("curl"));
        assert!(!is_command_safe("bash"));
        assert!(!is_command_safe("cmd.exe"));
        assert!(!is_command_safe("python3"));
    }

    #[tokio::test]
    async fn test_create_sandboxed_lua_restrictions() {
        let dir = tempdir().unwrap();
        let (tx, _rx) = tokio::sync::mpsc::channel(10);
        let lua = create_sandboxed_lua(dir.path(), false, tx).unwrap();

        // Standard restricted globals should be nil
        let globals = lua.globals();
        assert!(globals.get::<_, mlua::Value>("load").unwrap().is_nil());
        assert!(globals.get::<_, mlua::Value>("dofile").unwrap().is_nil());

        // io module should not exist
        assert!(globals.get::<_, mlua::Value>("io").unwrap().is_nil());
        // os module should not exist
        assert!(globals.get::<_, mlua::Value>("os").unwrap().is_nil());
    }
}
