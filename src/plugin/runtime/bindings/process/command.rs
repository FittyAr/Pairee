//! `pairee.Command` builder userdata.

use super::child::LuaChild;
use super::output::{LuaOutput, LuaStatus};
use super::stdio::StdioKind;
use crate::plugin::command_policy::CommandPolicy;
use mlua::{AnyUserData, Lua, MetaMethod, Table, UserData, UserDataMethods, Value};
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct LuaCommand {
    pub program: String,
    pub args: Vec<String>,
    pub cwd: Option<PathBuf>,
    pub env: Vec<(String, String)>,
    pub stdin: StdioKind,
    pub stdout: StdioKind,
    pub stderr: StdioKind,
    pub policy: Arc<CommandPolicy>,
}

impl LuaCommand {
    pub fn new(program: String, policy: Arc<CommandPolicy>) -> Self {
        Self {
            program,
            args: Vec::new(),
            cwd: None,
            env: Vec::new(),
            stdin: StdioKind::Inherit,
            stdout: StdioKind::Inherit,
            stderr: StdioKind::Inherit,
            policy,
        }
    }

    /// The program to execute (absolute in Secure Mode) or a violation.
    pub fn check_allowed(&self) -> mlua::Result<PathBuf> {
        self.policy
            .authorize(&self.program)
            .map_err(mlua::Error::RuntimeError)
    }

    fn tokio_cmd(&self, program: &Path) -> tokio::process::Command {
        let mut cmd = tokio::process::Command::new(program);
        cmd.args(&self.args);
        if let Some(cwd) = &self.cwd {
            cmd.current_dir(cwd);
        }
        for (k, v) in &self.env {
            cmd.env(k, v);
        }
        cmd.stdin(self.stdin.to_std());
        cmd.stdout(self.stdout.to_std());
        cmd.stderr(self.stderr.to_std());
        cmd
    }

    pub async fn spawn_inner(&self) -> mlua::Result<LuaChild> {
        let program = self.check_allowed()?;
        let mut child = self.tokio_cmd(&program).spawn().map_err(|e| {
            mlua::Error::RuntimeError(format!("Failed to spawn '{}': {e}", self.program))
        })?;
        Ok(LuaChild {
            stdin: child.stdin.take(),
            stdout: child.stdout.take(),
            stderr: child.stderr.take(),
            child: Some(child),
            leftover: Vec::new(),
        })
    }

    pub async fn output_inner(&self) -> mlua::Result<LuaOutput> {
        let program = self.check_allowed()?;
        let mut cmd = self.clone();
        cmd.stdout = StdioKind::Piped;
        cmd.stderr = StdioKind::Piped;
        let out = cmd.tokio_cmd(&program).output().await.map_err(|e| {
            mlua::Error::RuntimeError(format!("Failed to run '{}': {e}", self.program))
        })?;
        Ok(LuaOutput {
            status: LuaStatus::from_exit(out.status),
            stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
        })
    }

    pub async fn status_inner(&self) -> mlua::Result<LuaStatus> {
        let program = self.check_allowed()?;
        let status = self.tokio_cmd(&program).status().await.map_err(|e| {
            mlua::Error::RuntimeError(format!("Failed to run '{}': {e}", self.program))
        })?;
        Ok(LuaStatus::from_exit(status))
    }
}

fn chain(ud: AnyUserData) -> mlua::Result<AnyUserData> {
    Ok(ud)
}

impl UserData for LuaCommand {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_function("arg", |_, (ud, value): (AnyUserData, Value)| {
            {
                let mut this = ud.borrow_mut::<Self>()?;
                match value {
                    Value::String(s) => this.args.push(s.to_str()?.to_string()),
                    Value::Table(t) => {
                        for v in t.sequence_values::<String>() {
                            this.args.push(v?);
                        }
                    }
                    _ => {
                        return Err(mlua::Error::RuntimeError(
                            "Command:arg expects a string or list of strings".into(),
                        ));
                    }
                }
            }
            chain(ud)
        });

        methods.add_function("cwd", |_, (ud, path): (AnyUserData, String)| {
            ud.borrow_mut::<Self>()?.cwd = Some(PathBuf::from(path));
            chain(ud)
        });

        methods.add_function("env", |_, (ud, (k, v)): (AnyUserData, (String, String))| {
            ud.borrow_mut::<Self>()?.env.push((k, v));
            chain(ud)
        });

        methods.add_function("stdin", |_, (ud, v): (AnyUserData, Value)| {
            ud.borrow_mut::<Self>()?.stdin = StdioKind::from_lua(&v);
            chain(ud)
        });
        methods.add_function("stdout", |_, (ud, v): (AnyUserData, Value)| {
            ud.borrow_mut::<Self>()?.stdout = StdioKind::from_lua(&v);
            chain(ud)
        });
        methods.add_function("stderr", |_, (ud, v): (AnyUserData, Value)| {
            ud.borrow_mut::<Self>()?.stderr = StdioKind::from_lua(&v);
            chain(ud)
        });

        methods.add_async_method(
            "spawn",
            |_, this, ()| async move { this.spawn_inner().await },
        );
        methods.add_async_method(
            "output",
            |_, this, ()| async move { this.output_inner().await },
        );
        methods.add_async_method(
            "status",
            |_, this, ()| async move { this.status_inner().await },
        );

        methods.add_meta_method(MetaMethod::ToString, |_, this, ()| {
            Ok(format!("Command({})", this.program))
        });
    }
}

pub fn bind(lua: &Lua, policy: Arc<CommandPolicy>) -> mlua::Result<Table<'_>> {
    let table = lua.create_table()?;
    table.set("NULL", StdioKind::NULL as i64)?;
    table.set("PIPED", StdioKind::PIPED as i64)?;
    table.set("INHERIT", StdioKind::INHERIT as i64)?;

    let mt = lua.create_table()?;
    mt.set(
        "__call",
        lua.create_function(move |_, (_, program): (Table, String)| {
            Ok(LuaCommand::new(program, Arc::clone(&policy)))
        })?,
    )?;
    table.set_metatable(Some(mt));
    Ok(table)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy(trusted: bool, secure: bool, declared: &[&str]) -> Arc<CommandPolicy> {
        let declared: Vec<String> = declared.iter().map(|s| s.to_string()).collect();
        Arc::new(CommandPolicy::new(trusted, secure, &declared))
    }

    fn echo() -> (String, Vec<String>) {
        if cfg!(windows) {
            ("cmd.exe".into(), vec!["/C".into(), "echo hello".into()])
        } else {
            ("echo".into(), vec!["hello".into()])
        }
    }

    #[test]
    fn builder_accumulates_args_cwd_env() {
        let mut cmd = LuaCommand::new("ls".into(), policy(true, false, &[]));
        cmd.args.push("-l".into());
        cmd.cwd = Some(PathBuf::from("/tmp"));
        cmd.env.push(("FOO".into(), "bar".into()));
        cmd.stdout = StdioKind::Piped;
        assert_eq!(cmd.args, vec!["-l"]);
        assert_eq!(cmd.cwd.unwrap(), PathBuf::from("/tmp"));
        assert_eq!(cmd.env[0], ("FOO".into(), "bar".into()));
        assert_eq!(cmd.stdout, StdioKind::Piped);
    }

    #[test]
    fn untrusted_command_is_rejected() {
        let cmd = LuaCommand::new("echo".into(), policy(false, false, &[]));
        let err = cmd.check_allowed().unwrap_err();
        assert!(err.to_string().contains("sandboxed"));
    }

    #[test]
    fn secure_mode_requires_manifest_declaration() {
        let cmd = LuaCommand::new("curl".into(), policy(true, true, &["curl"]));
        assert!(cmd.check_allowed().is_err());
        let undeclared = LuaCommand::new("rg".into(), policy(true, true, &[]));
        let err = undeclared.check_allowed().unwrap_err();
        assert!(err.to_string().contains("not declared"), "{err}");
    }

    #[tokio::test]
    async fn output_captures_echo() {
        let (prog, args) = echo();
        let mut cmd = LuaCommand::new(prog, policy(true, false, &[]));
        cmd.args = args;
        let out = cmd.output_inner().await.unwrap();
        assert!(out.status.success);
        assert!(out.stdout.to_lowercase().contains("hello"));
    }

    #[test]
    fn lua_call_constructs_and_chains() {
        let lua = Lua::new();
        lua.globals()
            .set("Command", bind(&lua, policy(true, false, &[])).unwrap())
            .unwrap();
        let prog: String = lua
            .load(r#"return tostring(Command("rg"):arg("-n"):arg({"a"}))"#)
            .eval()
            .unwrap();
        assert_eq!(prog, "Command(rg)");
        let n: i64 = lua
            .load(
                r#"
                local c = Command("x"):arg("a"):arg({"b", "c"})
                return 1
                "#,
            )
            .eval()
            .unwrap();
        assert_eq!(n, 1);
    }
}
