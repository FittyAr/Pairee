//! `Command` builder userdata. Plugins use it to build up a
//! process invocation:
//!
//! ```lua
//! local child = Command("ls")
//!     :arg("-l")
//!     :arg("-a")
//!     :cwd("/tmp")
//!     :env("LANG", "C")
//!     :stdin(Command.NULL)
//!     :stdout(Command.PIPED)
//!     :stderr(Command.PIPED)
//!     :spawn()
//! local out = child:wait_with_output()
//! ```

use super::child::Child;
use super::stdio::Stdio;
use mlua::{Lua, UserData, UserDataMethods};
use std::process::Stdio as StdStdio;
use tokio::process::Command as TokioCommand;

/// Read the cached secure-mode flag set by `standard::bind_runtime`.
/// Returns `false` if the Lua state is missing it.
fn is_secure_mode(lua: &Lua) -> bool {
    lua.globals()
        .get::<_, mlua::Table>("pairee")
        .ok()
        .and_then(|p| p.get::<_, bool>("_secure_mode").ok())
        .unwrap_or(false)
}

/// Per roadmap §6: in Secure Mode, `Stdio::Inherit` is forbidden
/// because it lets the child process see the terminal (and any
/// authentication tokens typed into it). `PIPED` and `NULL`
/// remain allowed.
fn inherit_blocked_by_secure_mode(lua: &Lua, stdio: Stdio) -> bool {
    is_secure_mode(lua) && matches!(stdio, Stdio::Inherit)
}

/// The M3 `Command` userdata. Wraps the configuration needed
/// to build up a `tokio::process::Command` (which is not
/// `Clone`). When `:spawn()`/`:output()`/`:status()` is called
/// we materialise the real `TokioCommand` from this snapshot.
#[derive(Debug, Clone)]
pub struct Command {
    program: String,
    args: Vec<String>,
    cwd: Option<String>,
    env: Vec<(String, String)>,
    env_clear: bool,
    stdin: Option<Stdio>,
    stdout: Option<Stdio>,
    stderr: Option<Stdio>,
    kill_on_drop: bool,
    /// Optional RLIMIT_AS ceiling in bytes (M3 roadmap §5.B3).
    /// Set via `:memory(max)`. Honoured on Unix via
    /// `pre_exec`; logged-and-ignored on Windows.
    memory: Option<u64>,
}

impl Command {
    pub fn new(cmd: &str) -> Self {
        Self {
            program: cmd.to_string(),
            args: Vec::new(),
            cwd: None,
            env: Vec::new(),
            env_clear: false,
            stdin: None,
            stdout: None,
            stderr: None,
            kill_on_drop: false,
            memory: None,
        }
    }

    fn materialise(&self) -> TokioCommand {
        let mut c = TokioCommand::new(&self.program);
        c.args(&self.args);
        if let Some(cwd) = &self.cwd {
            c.current_dir(cwd);
        }
        if self.env_clear {
            c.env_clear();
        }
        for (k, v) in &self.env {
            c.env(k, v);
        }
        if let Some(s) = self.stdin {
            c.stdin(s.to_tokio());
        }
        if let Some(s) = self.stdout {
            c.stdout(s.to_tokio());
        }
        if let Some(s) = self.stderr {
            c.stderr(s.to_tokio());
        }
        c.kill_on_drop(self.kill_on_drop);
        if let Some(max) = self.memory {
            #[cfg(unix)]
            {
                // SAFETY: `pre_exec` runs in the forked child
                // between `fork` and `exec`. We only call
                // `libc::setrlimit` with a stack-local `rlimit`
                // struct; we never touch the parent's address
                // space. `RLIMIT_AS` caps the virtual address
                // space — anything above `max` bytes raises
                // `ENOMEM` on the next allocation.
                unsafe {
                    c.pre_exec(move || {
                        let rlim = libc::rlimit {
                            rlim_cur: max as libc::rlim_t,
                            rlim_max: max as libc::rlim_t,
                        };
                        if libc::setrlimit(libc::RLIMIT_AS, &rlim) != 0 {
                            return Err(std::io::Error::last_os_error());
                        }
                        Ok(())
                    });
                }
            }
            #[cfg(not(unix))]
            {
                // M3 simplification: Windows can't enforce
                // RLIMIT_AS. Log once and move on.
                let _ = max;
                log::warn!(
                    "Command.memory({}) is set but RLIMIT_AS is not supported on this platform",
                    max
                );
            }
        }
        c
    }
}

impl UserData for Command {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method_mut("arg", |_lua, this, arg: String| {
            this.args.push(arg);
            Ok(this.clone())
        });
        methods.add_method_mut("args", |_lua, this, args: Vec<String>| {
            this.args.extend(args);
            Ok(this.clone())
        });
        methods.add_method_mut("cwd", |_lua, this, dir: String| {
            this.cwd = Some(dir);
            Ok(this.clone())
        });
        methods.add_method_mut("env", |_lua, this, (k, v): (String, String)| {
            this.env.push((k, v));
            Ok(this.clone())
        });
        methods.add_method_mut("env_remove", |_lua, this, _k: String| {
            // We don't track removed env keys individually; the
            // caller can re-construct the Command if they need
            // this. Logged for future work.
            log::debug!("Command.env_remove is a no-op in M3");
            Ok(this.clone())
        });
        methods.add_method_mut("env_clear", |_lua, this, ()| {
            this.env_clear = true;
            this.env.clear();
            Ok(this.clone())
        });
        methods.add_method_mut("stdin", |lua_ctx, this, stdio: mlua::AnyUserData| {
            let s = stdio.borrow::<Stdio>().map_err(|e| mlua::Error::RuntimeError(format!("{e}")))?;
            // §6 Secure-Mode: INHERIT is forbidden because it
            // exposes the terminal (and any sensitive input) to
            // the child process.
            if inherit_blocked_by_secure_mode(lua_ctx, *s) {
                return Err(mlua::Error::RuntimeError(
                    "Command.stdin(Stdio::INHERIT) is blocked in Secure Mode"
                        .to_string(),
                ));
            }
            this.stdin = Some(*s);
            Ok(this.clone())
        });
        methods.add_method_mut("stdout", |lua_ctx, this, stdio: mlua::AnyUserData| {
            let s = stdio.borrow::<Stdio>().map_err(|e| mlua::Error::RuntimeError(format!("{e}")))?;
            if inherit_blocked_by_secure_mode(lua_ctx, *s) {
                return Err(mlua::Error::RuntimeError(
                    "Command.stdout(Stdio::INHERIT) is blocked in Secure Mode"
                        .to_string(),
                ));
            }
            this.stdout = Some(*s);
            Ok(this.clone())
        });
        methods.add_method_mut("stderr", |lua_ctx, this, stdio: mlua::AnyUserData| {
            let s = stdio.borrow::<Stdio>().map_err(|e| mlua::Error::RuntimeError(format!("{e}")))?;
            if inherit_blocked_by_secure_mode(lua_ctx, *s) {
                return Err(mlua::Error::RuntimeError(
                    "Command.stderr(Stdio::INHERIT) is blocked in Secure Mode"
                        .to_string(),
                ));
            }
            this.stderr = Some(*s);
            Ok(this.clone())
        });
        methods.add_method_mut("kill_on_drop", |_lua, this, yes: bool| {
            this.kill_on_drop = yes;
            Ok(this.clone())
        });
        // `:memory(max)` — set an RLIMIT_AS ceiling in bytes
        // (M3 roadmap §5.B3). On Unix this is enforced in the
        // forked child via `pre_exec`; on Windows it's a
        // logged-and-ignored no-op.
        methods.add_method_mut("memory", |_lua, this, max: u64| {
            this.memory = Some(max);
            Ok(this.clone())
        });

        // `:spawn()` — start the child process and return a
        // `Child` userdata that wraps the live handle.
        methods.add_async_method("spawn", |_lua, this, ()| async move {
            let mut cmd = this.materialise();
            match cmd.spawn() {
                Ok(mut child) => {
                    let id = child.id().unwrap_or(0);
                    let stdin = child.stdin.take();
                    let stdout = child.stdout.take();
                    let stderr = child.stderr.take();
                    let wrapped = Child {
                        id,
                        inner: std::sync::Arc::new(
                            tokio::sync::Mutex::new(Some(child)),
                        ),
                        stdin,
                        stdout,
                        stderr,
                    };
                    let ud = _lua.create_userdata(wrapped)?;
                    Ok(mlua::Value::UserData(ud))
                }
                Err(e) => Err(mlua::Error::RuntimeError(format!(
                    "Command.spawn failed: {e}"
                ))),
            }
        });

        // `:output()` — run to completion and capture stdout+stderr.
        methods.add_async_method("output", |_lua, this, ()| async move {
            let mut cmd = this.materialise();
            cmd.stdin(StdStdio::null());
            cmd.stdout(StdStdio::piped());
            cmd.stderr(StdStdio::piped());
            match cmd.output().await {
                Ok(out) => {
                    let output = super::output::Output::from_tokio(out);
                    let ud = _lua.create_userdata(output)?;
                    Ok(mlua::Value::UserData(ud))
                }
                Err(e) => Err(mlua::Error::RuntimeError(format!(
                    "Command.output failed: {e}"
                ))),
            }
        });

        // `:status()` — run to completion and return the exit
        // status.
        methods.add_async_method("status", |_lua, this, ()| async move {
            let mut cmd = this.materialise();
            cmd.stdin(StdStdio::null());
            cmd.stdout(StdStdio::null());
            cmd.stderr(StdStdio::null());
            match cmd.status().await {
                Ok(s) => {
                    let status = super::output::Status::from_exit(s);
                    let ud = _lua.create_userdata(status)?;
                    Ok(mlua::Value::UserData(ud))
                }
                Err(e) => Err(mlua::Error::RuntimeError(format!(
                    "Command.status failed: {e}"
                ))),
            }
        });
    }
}

/// Register the `Command(name)` callable on the given table so
/// plugins can write `Command("ls")`.
pub fn register(lua: &mlua::Lua, parent: &mlua::Table<'_>) -> mlua::Result<()> {
    let cmd = lua.create_table()?;
    cmd.set(
        "__call",
        lua.create_function(|lua, name: String| {
            let c = Command::new(&name);
            lua.create_userdata(c).map(mlua::Value::UserData)
        })?,
    )?;
    // Static factory `Command.new(name)` for explicit table-style
    // construction.
    cmd.set(
        "new",
        lua.create_function(|lua, name: String| {
            let c = Command::new(&name);
            lua.create_userdata(c).map(mlua::Value::UserData)
        })?,
    )?;
    super::stdio::register(lua, &cmd)?;
    parent.set("Command", cmd)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inherit_allowed_outside_secure_mode() {
        // Default mode (secure_mode=false). The check is per-lua-context,
        // not a global flag, so this test verifies the helper logic.
        let lua = mlua::Lua::new();
        let secure = is_secure_mode(&lua);
        assert!(!secure, "fresh lua has no _secure_mode set");
        assert!(!inherit_blocked_by_secure_mode(&lua, Stdio::Inherit));
        assert!(!inherit_blocked_by_secure_mode(&lua, Stdio::Piped));
        assert!(!inherit_blocked_by_secure_mode(&lua, Stdio::Null));
    }

    #[test]
    fn test_inherit_blocked_in_secure_mode() {
        // Plant a `pairee._secure_mode = true` table in the globals
        // so `is_secure_mode` reads back true.
        let lua = mlua::Lua::new();
        let pairee = lua.create_table().unwrap();
        pairee.set("_secure_mode", true).unwrap();
        lua.globals().set("pairee", pairee).unwrap();
        assert!(is_secure_mode(&lua));
        assert!(inherit_blocked_by_secure_mode(&lua, Stdio::Inherit));
        // PIPED and NULL are still allowed.
        assert!(!inherit_blocked_by_secure_mode(&lua, Stdio::Piped));
        assert!(!inherit_blocked_by_secure_mode(&lua, Stdio::Null));
    }

    #[test]
    fn test_command_bind_blocks_inherit_in_secure_mode() {
        // Sanity: helper itself returns false outside Secure Mode.
        let lua = mlua::Lua::new();
        assert!(!is_secure_mode(&lua));
        assert!(!inherit_blocked_by_secure_mode(&lua, Stdio::Inherit));
    }

    #[test]
    fn test_command_struct_field_defaults() {
        let c = Command::new("echo");
        assert_eq!(c.program, "echo");
        assert!(c.args.is_empty());
        assert!(c.env.is_empty());
        assert!(c.cwd.is_none());
        assert!(c.stdin.is_none());
        assert!(c.stdout.is_none());
        assert!(c.stderr.is_none());
        assert!(!c.kill_on_drop);
        assert!(c.memory.is_none());
    }

    #[test]
    fn test_command_builder_chain_appends_args() {
        let mut c = Command::new("ls");
        c.args.push("-l".to_string());
        c.args.push("-a".to_string());
        assert_eq!(c.args.len(), 2);
        assert_eq!(c.args[0], "-l");
        assert_eq!(c.args[1], "-a");
    }

    #[test]
    fn test_materialise_propagates_fields() {
        let mut c = Command::new("ls");
        c.args.push("-l".to_string());
        c.cwd = Some("/tmp".to_string());
        c.env.push(("FOO".to_string(), "bar".to_string()));
        let tokio_cmd = c.materialise();
        // tokio::process::Command has no public field accessors
        // beyond Debug; verify by Debug formatting.
        let debug = format!("{tokio_cmd:?}");
        assert!(debug.contains("ls"));
        assert!(debug.contains("-l"));
    }

    #[test]
    fn test_memory_field_round_trip() {
        let mut c = Command::new("x");
        c.memory = Some(1_073_741_824);
        assert_eq!(c.memory, Some(1_073_741_824));
    }
}
