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
use mlua::{UserData, UserDataMethods};
use std::process::Stdio as StdStdio;
use tokio::process::Command as TokioCommand;

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
        methods.add_method_mut("stdin", |_lua, this, stdio: mlua::AnyUserData| {
            let s = stdio.borrow::<Stdio>().map_err(|e| mlua::Error::RuntimeError(format!("{e}")))?;
            this.stdin = Some(*s);
            Ok(this.clone())
        });
        methods.add_method_mut("stdout", |_lua, this, stdio: mlua::AnyUserData| {
            let s = stdio.borrow::<Stdio>().map_err(|e| mlua::Error::RuntimeError(format!("{e}")))?;
            this.stdout = Some(*s);
            Ok(this.clone())
        });
        methods.add_method_mut("stderr", |_lua, this, stdio: mlua::AnyUserData| {
            let s = stdio.borrow::<Stdio>().map_err(|e| mlua::Error::RuntimeError(format!("{e}")))?;
            this.stderr = Some(*s);
            Ok(this.clone())
        });
        methods.add_method_mut("kill_on_drop", |_lua, this, yes: bool| {
            this.kill_on_drop = yes;
            Ok(this.clone())
        });

        // `:spawn()` — start the child process and return a
        // `Child` userdata that wraps the live handle.
        methods.add_async_method("spawn", |_lua, this, ()| async move {
            let mut cmd = this.materialise();
            match cmd.spawn() {
                Ok(child) => {
                    let id = child.id().unwrap_or(0);
                    let stdin = child.stdin.take();
                    let stdout = child.stdout.take();
                    let stderr = child.stderr.take();
                    let wrapped = Child {
                        id,
                        inner: Some(child),
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
