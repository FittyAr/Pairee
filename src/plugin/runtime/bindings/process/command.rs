//! `pairee.Command` builder userdata.

use super::capture::{self, MAX_TIMEOUT, OUTPUT_TIMEOUT, STATUS_TIMEOUT};
use super::child::LuaChild;
use super::output::{LuaOutput, LuaStatus};
use super::stdio::StdioKind;
use crate::plugin::command_policy::{CommandPolicy, PathUse};
use crate::plugin::runtime::bindings::fs::{Access, FsPolicy, Target};
use mlua::{AnyUserData, Lua, MetaMethod, Table, UserData, UserDataMethods, Value};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

#[derive(Clone, Debug)]
pub struct LuaCommand {
    pub program: String,
    pub args: Vec<String>,
    pub cwd: Option<PathBuf>,
    pub env: Vec<(String, String)>,
    pub stdin: StdioKind,
    pub stdout: StdioKind,
    pub stderr: StdioKind,
    /// `Command:timeout(seconds)`; `None`: the default of each call.
    pub timeout: Option<Duration>,
    pub policy: Arc<CommandPolicy>,
    /// The plugin's `pairee.fs` jail; in Secure Mode `cwd` must lie inside.
    pub sandbox: Arc<FsPolicy>,
}

impl LuaCommand {
    pub fn new(program: String, policy: Arc<CommandPolicy>, sandbox: Arc<FsPolicy>) -> Self {
        Self {
            program,
            args: Vec::new(),
            cwd: None,
            env: Vec::new(),
            // A plugin's child never reads the user's keyboard unless the
            // plugin asks for it with `:stdin(INHERIT)`.
            stdin: StdioKind::Null,
            stdout: StdioKind::Inherit,
            stderr: StdioKind::Inherit,
            timeout: None,
            policy,
            sandbox,
        }
    }

    /// Records `name=value` for the child, if the policy allows the name.
    pub fn set_env(&mut self, name: String, value: String) -> mlua::Result<()> {
        self.policy
            .authorize_env(&name)
            .map_err(mlua::Error::RuntimeError)?;
        self.env.push((name, value));
        Ok(())
    }

    /// Sets the working directory. In Secure Mode it must be an existing
    /// folder inside the plugin's `pairee.fs` **write** jail (a tool run in
    /// the config folder could overwrite `config.toml` or other plugins);
    /// the resolved (canonical) folder is what the child starts in.
    pub fn set_cwd(&mut self, path: &str) -> mlua::Result<()> {
        if !self.policy.is_allowlist() {
            self.cwd = Some(PathBuf::from(path));
            return Ok(());
        }
        let dir = match self.sandbox.check(path, Access::Write) {
            Ok(Target::Jailed { root, rel, .. }) => root.join(rel),
            Ok(Target::Free(p)) => p,
            Err(e) => return Err(mlua::Error::RuntimeError(e)),
        };
        if !dir.is_dir() {
            return Err(mlua::Error::RuntimeError(format!(
                "Command:cwd: {path:?} is not an existing folder"
            )));
        }
        self.cwd = Some(dir);
        Ok(())
    }

    /// The program to execute (absolute in Secure Mode) and its arguments,
    /// or a violation.
    pub fn check_allowed(&self) -> mlua::Result<(PathBuf, Vec<String>)> {
        let allowed = self
            .policy
            .authorize_command(&self.program, &self.args)
            .map_err(mlua::Error::RuntimeError)?;
        self.check_path_arguments()?;
        Ok(allowed)
    }

    /// In Secure Mode, the paths the arguments make the tool write (`tar
    /// -C`, `git clone <dest>`, the archive `zip` creates) must lie where
    /// `pairee.fs` could write them; relative ones start at the command's
    /// working directory.
    fn check_path_arguments(&self) -> mlua::Result<()> {
        for (path, usage) in self.policy.path_arguments(&self.program, &self.args) {
            let absolute = match &self.cwd {
                Some(cwd) if !std::path::Path::new(&path).is_absolute() => {
                    cwd.join(&path).to_string_lossy().into_owned()
                }
                _ => path,
            };
            let access = match usage {
                PathUse::Read => Access::Read,
                PathUse::Write => Access::Write,
            };
            self.sandbox
                .check(&absolute, access)
                .map_err(mlua::Error::RuntimeError)?;
        }
        Ok(())
    }

    fn tokio_cmd(&self, (program, args): &(PathBuf, Vec<String>)) -> tokio::process::Command {
        let mut cmd = tokio::process::Command::new(program);
        cmd.args(args);
        // Dropping the handle (a plugin forgetting a Child, a cancelled
        // call) must not leave the program running.
        cmd.kill_on_drop(true);
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
        let run_error = |e: std::io::Error| {
            mlua::Error::RuntimeError(format!("Failed to run '{}': {e}", self.program))
        };
        let mut child = cmd.tokio_cmd(&program).spawn().map_err(run_error)?;
        let (status, stdout, stderr) =
            capture::collect(&mut child, self.time_limit(OUTPUT_TIMEOUT))
                .await
                .map_err(run_error)?;
        Ok(LuaOutput {
            status: LuaStatus::from_exit(status),
            stdout: String::from_utf8_lossy(&stdout).into_owned(),
            stderr: String::from_utf8_lossy(&stderr).into_owned(),
        })
    }

    pub async fn status_inner(&self) -> mlua::Result<LuaStatus> {
        let program = self.check_allowed()?;
        let run_error = |e: std::io::Error| {
            mlua::Error::RuntimeError(format!("Failed to run '{}': {e}", self.program))
        };
        let mut child = self.tokio_cmd(&program).spawn().map_err(run_error)?;
        let status = capture::wait_within(&mut child, self.time_limit(STATUS_TIMEOUT))
            .await
            .map_err(run_error)?;
        Ok(LuaStatus::from_exit(status))
    }

    /// The plugin's `:timeout()` (capped) or `default`.
    fn time_limit(&self, default: Duration) -> Duration {
        self.timeout.map_or(default, |t| t.min(MAX_TIMEOUT))
    }
}

fn chain(ud: AnyUserData) -> mlua::Result<AnyUserData> {
    Ok(ud)
}

impl UserData for LuaCommand {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
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
            ud.borrow_mut::<Self>()?.set_cwd(&path)?;
            chain(ud)
        });

        methods.add_function("env", |_, (ud, (k, v)): (AnyUserData, (String, String))| {
            ud.borrow_mut::<Self>()?.set_env(k, v)?;
            chain(ud)
        });

        methods.add_function("timeout", |_, (ud, seconds): (AnyUserData, f64)| {
            if !seconds.is_finite() || seconds <= 0.0 {
                return Err(mlua::Error::RuntimeError(
                    "Command:timeout expects a positive number of seconds".into(),
                ));
            }
            ud.borrow_mut::<Self>()?.timeout = Some(Duration::from_secs_f64(seconds.min(1e9)));
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

pub fn bind(lua: &Lua, policy: Arc<CommandPolicy>, sandbox: Arc<FsPolicy>) -> mlua::Result<Table> {
    let table = lua.create_table()?;
    table.set("NULL", StdioKind::NULL as i64)?;
    table.set("PIPED", StdioKind::PIPED as i64)?;
    table.set("INHERIT", StdioKind::INHERIT as i64)?;

    let mt = lua.create_table()?;
    mt.set(
        "__call",
        lua.create_function(move |_, (_, program): (Table, String)| {
            Ok(LuaCommand::new(
                program,
                Arc::clone(&policy),
                Arc::clone(&sandbox),
            ))
        })?,
    )?;
    table.set_metatable(Some(mt))?;
    Ok(table)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy(trusted: bool, secure: bool, declared: &[&str]) -> Arc<CommandPolicy> {
        let declared: Vec<String> = declared.iter().map(|s| s.to_string()).collect();
        Arc::new(CommandPolicy::new(trusted, secure, &declared))
    }

    fn sandbox(trusted: bool, secure: bool) -> Arc<FsPolicy> {
        use std::path::Path;
        let mut policy = FsPolicy::new(Path::new("demo"), trusted, secure);
        policy.workspace = std::env::current_dir().unwrap();
        Arc::new(policy)
    }

    fn cmd(program: &str, trusted: bool, secure: bool, declared: &[&str]) -> LuaCommand {
        LuaCommand::new(
            program.into(),
            policy(trusted, secure, declared),
            sandbox(trusted, secure),
        )
    }

    #[test]
    fn secure_mode_refuses_dangerous_environment() {
        let mut c = cmd("git", true, true, &["git"]);
        for name in ["GIT_SSH_COMMAND", "LD_PRELOAD", "PAGER", "NODE_OPTIONS"] {
            assert!(c.set_env(name.into(), "x".into()).is_err(), "{name}");
        }
        c.set_env("LANG".into(), "C".into()).unwrap();
        assert_eq!(c.env, vec![("LANG".to_string(), "C".to_string())]);
    }

    #[test]
    fn secure_mode_cwd_must_be_inside_the_jail() {
        let mut c = cmd("git", true, true, &["git"]);
        let outside = tempfile::tempdir().unwrap();
        let err = c.set_cwd(&outside.path().to_string_lossy()).unwrap_err();
        assert!(err.to_string().contains("Security violation"), "{err}");
        let cwd = std::env::current_dir().unwrap();
        c.set_cwd(&cwd.to_string_lossy()).unwrap();
        assert!(c.cwd.unwrap().is_dir());
    }

    #[test]
    fn secure_mode_cwd_needs_write_access_so_config_is_refused() {
        let c = cmd("git", true, true, &["git"]);
        let config = crate::config::paths::get_config_dir();
        let err = c.clone().set_cwd(&config.to_string_lossy()).unwrap_err();
        assert!(err.to_string().contains("Security violation"), "{err}");
    }

    #[test]
    fn secure_mode_refuses_written_paths_outside_the_jail() {
        let outside = tempfile::tempdir().unwrap();
        let out = outside.path().to_string_lossy().into_owned();
        let work = std::env::current_dir().unwrap();
        let mut c = cmd("tar", true, true, &["tar", "git"]);
        c.args = vec!["-xf".into(), "a.tar".into(), "-C".into(), out.clone()];
        assert!(c.check_path_arguments().is_err());
        c.args = vec!["-xf".into(), "a.tar".into(), "-C".into(), "sub".into()];
        c.cwd = Some(work);
        assert!(c.check_path_arguments().is_ok());
        let mut git = cmd("git", true, true, &["git"]);
        git.args = vec!["clone".into(), "https://example.com/r.git".into(), out];
        assert!(git.check_path_arguments().is_err());
        let free = cmd("tar", true, false, &[]);
        assert!(free.check_path_arguments().is_ok());
    }

    #[test]
    fn child_stdin_defaults_to_null() {
        assert_eq!(cmd("ls", true, false, &[]).stdin, StdioKind::Null);
    }

    #[tokio::test]
    async fn a_command_past_its_timeout_is_killed() {
        let mut c = cmd(sleeper().0, true, false, &[]);
        c.args = sleeper().1;
        c.timeout = Some(Duration::from_millis(200));
        let err = c.output_inner().await.unwrap_err();
        assert!(err.to_string().contains("did not finish"), "{err}");
    }

    fn sleeper() -> (&'static str, Vec<String>) {
        if cfg!(windows) {
            ("ping", vec!["-n".into(), "30".into(), "127.0.0.1".into()])
        } else {
            ("sleep", vec!["30".into()])
        }
    }

    #[test]
    fn secure_mode_cwd_must_exist() {
        let mut c = cmd("git", true, true, &["git"]);
        let missing = std::env::current_dir().unwrap().join("no-such-dir-xyz");
        assert!(c.set_cwd(&missing.to_string_lossy()).is_err());
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
        let mut cmd = cmd("ls", true, false, &[]);
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
        let cmd = cmd("echo", false, false, &[]);
        let err = cmd.check_allowed().unwrap_err();
        assert!(err.to_string().contains("sandboxed"));
    }

    #[test]
    fn secure_mode_git_runs_without_implicit_bare_repositories() {
        let dir = tempfile::tempdir().unwrap();
        crate::plugin::command_policy::make_tool(dir.path(), "git");
        let path = std::env::join_paths([dir.path()]).unwrap();
        let policy = Arc::new(CommandPolicy::new(true, true, &["git".to_string()]));
        let (_, args) = policy
            .authorize_command_with_path("git", &["status".to_string()], &path)
            .unwrap();
        assert_eq!(args, ["-c", "safe.bareRepository=explicit", "status"]);
        let open = CommandPolicy::new(true, false, &[]);
        let (_, args) = open.authorize_command("git", &["status".into()]).unwrap();
        assert_eq!(args, ["status"]);
    }

    #[test]
    fn secure_mode_requires_manifest_declaration() {
        let curl = cmd("curl", true, true, &["curl"]);
        assert!(curl.check_allowed().is_err());
        let undeclared = cmd("rg", true, true, &[]);
        let err = undeclared.check_allowed().unwrap_err();
        assert!(err.to_string().contains("not declared"), "{err}");
    }

    #[tokio::test]
    async fn output_captures_echo() {
        let (prog, args) = echo();
        let mut cmd = cmd(&prog, true, false, &[]);
        cmd.args = args;
        let out = cmd.output_inner().await.unwrap();
        assert!(out.status.success);
        assert!(out.stdout.to_lowercase().contains("hello"));
    }

    #[test]
    fn lua_call_constructs_and_chains() {
        let lua = Lua::new();
        lua.globals()
            .set(
                "Command",
                bind(&lua, policy(true, false, &[]), sandbox(true, false)).unwrap(),
            )
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
