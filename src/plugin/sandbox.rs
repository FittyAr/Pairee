use crate::plugin::manager::PluginRequest;
use std::path::Path;
use tokio::sync::mpsc;

/// Executables that untrusted (Secure Mode) plugins may never spawn:
/// shells, script interpreters, LOLBins and wrappers that run another
/// command (which would trivially bypass this list).
const BLOCKED_COMMANDS: &[&str] = &[
    // Network clients
    "curl",
    "wget",
    "nc",
    "ncat",
    "netcat",
    "socat",
    "ssh",
    "scp",
    "sftp",
    "telnet",
    "ftp",
    "tftp",
    "rsync",
    "nmap",
    "bitsadmin",
    "certutil",
    // Unix shells and multi-call binaries
    "sh",
    "bash",
    "zsh",
    "csh",
    "tcsh",
    "ksh",
    "mksh",
    "dash",
    "ash",
    "fish",
    "busybox",
    "toybox",
    // Windows shells and script hosts
    "cmd",
    "powershell",
    "powershell_ise",
    "pwsh",
    "wscript",
    "cscript",
    "mshta",
    "rundll32",
    "regsvr32",
    "msiexec",
    "wmic",
    "schtasks",
    "forfiles",
    "msbuild",
    "installutil",
    "conhost",
    "explorer",
    // Script interpreters / runtimes
    "python",
    "pythonw",
    "py",
    "pyw",
    "perl",
    "ruby",
    "irb",
    "node",
    "nodejs",
    "deno",
    "bun",
    "php",
    "lua",
    "luajit",
    "tclsh",
    "wish",
    "expect",
    "java",
    "jshell",
    "osascript",
    "awk",
    "gawk",
    "mawk",
    "nawk",
    // Wrappers that execute their arguments as a command
    "env",
    "xargs",
    "find",
    "nohup",
    "nice",
    "timeout",
    "setsid",
    "stdbuf",
    "script",
    "sudo",
    "su",
    "doas",
    "pkexec",
    "runas",
    "start",
    "open",
];

/// Command-name prefixes that are always blocked (versioned interpreters
/// such as `python3.12`, `powershell_ise`, `perl5.36`).
const BLOCKED_PREFIXES: &[&str] = &[
    "python",
    "powershell",
    "pwsh",
    "perl",
    "ruby",
    "php",
    "node",
    "lua",
];

/// Extensions Windows resolves implicitly; stripped before matching so
/// `CMD.EXE`, `bash.cmd` or `node.bat` are treated like their base names.
const EXECUTABLE_EXTENSIONS: &[&str] = &[
    ".exe", ".com", ".cmd", ".bat", ".ps1", ".vbs", ".js", ".msc",
];

/// Reduces a program path to a lowercase base name without directory,
/// trailing dots/spaces (ignored by Windows) or executable extension.
fn normalize_command_name(cmd: &str) -> String {
    let file = cmd.rsplit(['/', '\\']).next().unwrap_or(cmd);
    let mut name = file
        .trim()
        .trim_end_matches(['.', ' '])
        .to_ascii_lowercase();
    while let Some(ext) = EXECUTABLE_EXTENSIONS
        .iter()
        .find(|ext| name.len() > ext.len() && name.ends_with(*ext))
    {
        name.truncate(name.len() - ext.len());
        name = name.trim_end_matches(['.', ' ']).to_string();
    }
    name
}

/// Secure Mode command policy for plugins. This is a deny list: it blocks
/// the obvious ways to run arbitrary code, but it is not a complete sandbox.
pub fn is_command_safe(cmd: &str) -> bool {
    let name = normalize_command_name(cmd);
    if name.is_empty() {
        return false;
    }
    let unversioned =
        name.trim_end_matches(|c: char| c.is_ascii_digit() || matches!(c, '.' | '-' | '_'));
    let blocked = BLOCKED_COMMANDS.contains(&name.as_str())
        || BLOCKED_COMMANDS.contains(&unversioned)
        || BLOCKED_PREFIXES.iter().any(|p| name.starts_with(p));
    !blocked
}

pub fn create_sandboxed_lua(
    plugin_dir: &Path,
    trusted: bool,
    tx: mpsc::Sender<PluginRequest>,
) -> anyhow::Result<mlua::Lua> {
    // 1. Determine standard libraries to load
    let std_libs = if trusted {
        // ALL includes `debug`, which `new_with` rejects. Trusted plugins
        // get io/os/package via ALL_SAFE (no debug/ffi).
        mlua::StdLib::ALL_SAFE
    } else {
        mlua::StdLib::TABLE | mlua::StdLib::STRING | mlua::StdLib::UTF8 | mlua::StdLib::MATH
    };

    let lua = mlua::Lua::new_with(std_libs, mlua::LuaOptions::default())?;
    crate::plugin::limits::apply(&lua, trusted);

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

    #[test]
    fn command_names_are_normalized() {
        for blocked in [
            "CMD.EXE",
            "C:\\Windows\\System32\\cmd.exe",
            "/usr/bin/bash",
            "bash.cmd",
            "PowerShell.exe",
            "pwsh",
            "python3.12",
            "pythonw.exe",
            "py.exe",
            "node.bat",
            "wscript.exe",
            "cscript",
            "mshta.exe",
            "rundll32.exe",
            "/usr/bin/env",
            "osascript",
            "busybox",
            "dash",
            "cmd.",
            "cmd.exe. ",
            "perl5.36",
            "",
        ] {
            assert!(!is_command_safe(blocked), "{blocked:?} should be blocked");
        }
        for allowed in ["git", "cargo", "ls", "C:\\tools\\rg.exe", "/usr/bin/fd"] {
            assert!(is_command_safe(allowed), "{allowed:?} should be allowed");
        }
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

    #[test]
    fn trusted_lua_loads_without_debug() {
        let dir = tempdir().unwrap();
        let (tx, _rx) = tokio::sync::mpsc::channel(4);
        let lua = create_sandboxed_lua(dir.path(), true, tx).unwrap();
        let globals = lua.globals();
        assert!(!globals.get::<_, mlua::Value>("pairee").unwrap().is_nil());
        assert!(!globals.get::<_, mlua::Value>("io").unwrap().is_nil());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn untrusted_fs_is_jailed_without_secure_mode() {
        let dir = tempdir().unwrap();
        let outside = tempdir().unwrap();
        let target = outside.path().join("pwn.txt");
        let (tx, _rx) = tokio::sync::mpsc::channel(4);
        let lua = create_sandboxed_lua(dir.path(), false, tx).unwrap();
        lua.globals()
            .set("target", target.to_string_lossy().to_string())
            .unwrap();
        let err = lua
            .load(r#"pairee.fs.write(target, "x")"#)
            .exec()
            .unwrap_err();
        assert!(err.to_string().contains("Security violation"), "{err}");
        assert!(!target.exists());
    }

    #[test]
    fn secure_mode_flag_is_not_lua_writable() {
        let dir = tempdir().unwrap();
        let outside = tempdir().unwrap();
        let target = outside.path().join("pwn.txt");
        let (tx, _rx) = tokio::sync::mpsc::channel(4);
        let lua = create_sandboxed_lua(dir.path(), false, tx).unwrap();
        lua.globals()
            .set("target", target.to_string_lossy().to_string())
            .unwrap();
        let result = lua
            .load(r#"pairee._secure_mode = false; pairee.fs.write(target, "x")"#)
            .exec();
        assert!(result.is_err());
        assert!(!target.exists());
    }
}
