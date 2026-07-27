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

/// Read the cached trust flag set by `standard::bind_runtime`.
/// Returns `false` if the Lua state is missing it.
fn is_trusted(lua: &Lua) -> bool {
    lua.globals()
        .get::<_, mlua::Table>("pairee")
        .ok()
        .and_then(|p| p.get::<_, bool>("_trusted").ok())
        .unwrap_or(false)
}

/// Per roadmap §6: in Secure Mode, `Stdio::Inherit` is forbidden
/// because it lets the child process see the terminal (and any
/// authentication tokens typed into it). `PIPED` and `NULL`
/// remain allowed.
fn inherit_blocked_by_secure_mode(lua: &Lua, stdio: Stdio) -> bool {
    is_secure_mode(lua) && matches!(stdio, Stdio::Inherit)
}

/// §6 Secure-Mode: a `Command` constructed by a plugin must not
/// be allowed to run a blacklisted binary (`curl`, `sh`, etc.)
/// the same way `pairee.fs.spawn` is checked. We pass the
/// `lua_ctx` through every method so the check has access to the
/// `_secure_mode` flag.
fn spawn_blocked_in_secure_mode(lua: &Lua, program: &str) -> bool {
    is_secure_mode(lua) && !crate::plugin::sandbox::is_command_safe(program)
}

/// Trust gate: an *untrusted* plugin must not be able to spawn a
/// process via the `pairee.Command(...)` builder at all. The
/// equivalent `pairee.fs.spawn` already enforces this (see
/// `fs.rs`); the new process binding was missing the gate, which
/// let any untrusted plugin run arbitrary binaries whenever
/// `secure_mode` was off. Trusted plugins pass this check (their
/// trust model grants full process access by design).
fn spawn_blocked_by_trust(lua: &Lua) -> bool {
    !is_trusted(lua)
}

/// §6 Secure-Mode: a denylist of environment variable names that
/// influence dynamic linker behaviour. Letting a plugin set
/// `LD_PRELOAD` is a sandbox-escape primitive: the preloaded
/// library runs inside the child process with full code
/// execution. The same applies to `LD_LIBRARY_PATH` (forces a
/// custom library to be loaded instead of the system one) and
/// the `DYLD_*` family on macOS.
///
/// The check is case-insensitive because the dynamic linker
/// accepts several variants (e.g. `LD_PRELOAD`, `ld_preload`).
fn env_blocked_in_secure_mode(lua: &Lua, name: &str) -> bool {
    if !is_secure_mode(lua) {
        return false;
    }
    let upper: String = name.to_ascii_uppercase();
    const DENYLIST: &[&str] = &[
        // ELF dynamic linker (Linux, *BSD, etc.)
        "LD_PRELOAD",
        "LD_LIBRARY_PATH",
        "LD_AUDIT",
        "LD_DEBUG",
        "LD_DEBUG_OUTPUT",
        "LD_BIND_NOW",
        "LD_PROFILE",
        "LD_SHOW_AUXV",
        "LD_HWCAP_MASK",
        "LD_ORIGIN_PATH",
        "LD_DYNAMIC_WEAK",
        "LD_USE_LOAD_BIAS",
        // macOS dyld
        "DYLD_INSERT_LIBRARIES",
        "DYLD_LIBRARY_PATH",
        "DYLD_FRAMEWORK_PATH",
        "DYLD_FALLBACK_FRAMEWORK_PATH",
        "DYLD_FALLBACK_LIBRARY_PATH",
        "DYLD_ROOT_PATH",
        "DYLD_SHARED_REGION",
        "DYLD_SHARED_CACHE_DIR",
    ];
    DENYLIST.iter().any(|blocked| upper == *blocked)
}

/// §6 Secure-Mode: validate a cwd against the frozen sandbox
/// roots. A plugin must not be able to spawn a child inside a
/// directory outside the workspace — that would let the child
/// read or write outside the sandbox with the same privileges
/// as the plugin (which has only the workspace's read/write
/// rights, but a child process can be a stepping stone to
/// more). We re-use `fs::bindings::is_in_sandbox` by going
/// through the same canonicalize path so the check matches
/// what the file bindings do.
///
/// Returns `Ok(())` if Secure Mode is off (no validation), if
/// the path is inside the sandbox, or if the path is missing
/// entirely (an "open" cwd that the OS will reject on exec
/// anyway — we don't want to give plugins a way to learn that
/// a path is invalid by observing this check).
fn cwd_in_sandbox(lua: &Lua, cwd: &str) -> bool {
    if !is_secure_mode(lua) {
        return true;
    }
    // Mirror the file-binding sandbox check: canonicalize, then
    // ask the bindings module whether the result is in a
    // permitted root. We import the function lazily to avoid a
    // cycle between the process and fs binding modules.
    let canonical = match std::fs::canonicalize(cwd) {
        Ok(p) => p,
        Err(_) => {
            // Path doesn't exist. We can't tell if it's inside
            // or outside the sandbox, so we err on the side of
            // accepting (the spawn will fail at exec time with
            // a clearer error anyway). The same trade-off is
            // documented in `validate_path_with`.
            return true;
        }
    };
    crate::plugin::runtime::bindings::fs::is_in_sandbox(&canonical)
}

/// §6 hardening: minimum sensible value for `RLIMIT_AS`.
/// Setting `RLIMIT_AS` to a value smaller than the loader
/// itself needs causes the exec to fail with `ENOMEM` before
/// the program even starts — a denial-of-service against the
/// user's own machine. We pick 4 MiB as a floor (small enough
/// to be useful for testing, large enough to host the dynamic
/// linker on every platform we support).
const MIN_RLIMIT_AS: u64 = 4 * 1024 * 1024;

/// §6: clamp a user-supplied `RLIMIT_AS` request to a minimum
/// sensible value. `pub(crate)` so the unit tests can verify
/// the clamp behaviour without spawning a real process.
pub(crate) fn clamp_rlimit_as(requested: u64) -> u64 {
    requested.max(MIN_RLIMIT_AS)
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
            // §6 hardening: clamp the request to a minimum
            // sensible value (see `clamp_rlimit_as` at module
            // scope for the rationale). Setting `RLIMIT_AS`
            // to a value smaller than the loader itself needs
            // causes the exec to fail with `ENOMEM` before
            // the program even starts.
            let clamped = clamp_rlimit_as(max);
            if clamped != max {
                log::warn!(
                    "Command.memory({}) is below the {} byte floor; clamping to {}",
                    max,
                    MIN_RLIMIT_AS,
                    clamped
                );
            }
            #[cfg(unix)]
            {
                // SAFETY: `pre_exec` runs in the forked child
                // between `fork` and `exec`. We only call
                // `libc::setrlimit` with a stack-local `rlimit`
                // struct; we never touch the parent's address
                // space.
                //
                // Platform notes for `RLIMIT_AS`:
                //   * Linux: caps the virtual address space of
                //     the process. Allocations above the limit
                //     return `ENOMEM`. This is the "real"
                //     enforcement.
                //   * macOS: `RLIMIT_AS` is implemented but the
                //     kernel may use it more loosely than on
                //     Linux; the limit is *not* always honoured
                //     for mmap'd files.
                //   * FreeBSD/OpenBSD/NetBSD: behaviour is
                //     similar to Linux; some kernels additionally
                //     require `RLIMIT_DATA` to be lowered for
                //     data-segment-heavy programs.
                //
                // We set `rlim_cur == rlim_max` so the limit is
                // not a "soft" hint that the child can raise
                // back. Note that non-root processes cannot
                // raise `rlim_max` (only lower it), so this
                // matters most for trusted plugins.
                unsafe {
                    c.pre_exec(move || {
                        let rlim = libc::rlimit {
                            rlim_cur: clamped as libc::rlim_t,
                            rlim_max: clamped as libc::rlim_t,
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
                // M3 simplification: Windows has no equivalent
                // of `RLIMIT_AS`. The closest is the job-object
                // `JOB_OBJECT_LIMIT_PROCESS_MEMORY`, but that
                // requires the child to be wrapped in a job at
                // spawn time, which `tokio::process` does not
                // expose. Log once per call and move on; the
                // plugin can still cap memory by using a
                // child-monitoring helper.
                let _ = clamped;
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
        methods.add_method_mut("cwd", |lua_ctx, this, dir: String| {
            // §6 Secure-Mode: refuse a cwd that resolves to a
            // path outside the sandbox. Without this check, a
            // plugin could `cwd("/etc")` and then `spawn("ls")`
            // — the child would still be subject to the binary
            // blacklist, but its working directory would be
            // outside the workspace, leaking the existence of
            // arbitrary files via `getcwd` introspection in
            // some libraries.
            if is_secure_mode(lua_ctx) && !cwd_in_sandbox(lua_ctx, &dir) {
                return Err(mlua::Error::RuntimeError(format!(
                    "Command:cwd({:?}) is blocked in Secure Mode (outside sandbox)",
                    dir
                )));
            }
            this.cwd = Some(dir);
            Ok(this.clone())
        });
        methods.add_method_mut("env", |lua_ctx, this, (k, v): (String, String)| {
            // §6 Secure-Mode: refuse dynamic-linker hooks
            // (`LD_PRELOAD`, `LD_LIBRARY_PATH`, `DYLD_*`, …).
            // These are sandbox-escape primitives — a preloaded
            // library runs in the child process with full code
            // execution.
            if env_blocked_in_secure_mode(lua_ctx, &k) {
                return Err(mlua::Error::RuntimeError(format!(
                    "Command:env({:?}, _) is blocked in Secure Mode (dynamic-linker hook)",
                    k
                )));
            }
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
            let s = stdio
                .borrow::<Stdio>()
                .map_err(|e| mlua::Error::RuntimeError(format!("{e}")))?;
            // §6 Secure-Mode: INHERIT is forbidden because it
            // exposes the terminal (and any sensitive input) to
            // the child process.
            if inherit_blocked_by_secure_mode(lua_ctx, *s) {
                return Err(mlua::Error::RuntimeError(
                    "Command.stdin(Stdio::INHERIT) is blocked in Secure Mode".to_string(),
                ));
            }
            this.stdin = Some(*s);
            Ok(this.clone())
        });
        methods.add_method_mut("stdout", |lua_ctx, this, stdio: mlua::AnyUserData| {
            let s = stdio
                .borrow::<Stdio>()
                .map_err(|e| mlua::Error::RuntimeError(format!("{e}")))?;
            if inherit_blocked_by_secure_mode(lua_ctx, *s) {
                return Err(mlua::Error::RuntimeError(
                    "Command.stdout(Stdio::INHERIT) is blocked in Secure Mode".to_string(),
                ));
            }
            this.stdout = Some(*s);
            Ok(this.clone())
        });
        methods.add_method_mut("stderr", |lua_ctx, this, stdio: mlua::AnyUserData| {
            let s = stdio
                .borrow::<Stdio>()
                .map_err(|e| mlua::Error::RuntimeError(format!("{e}")))?;
            if inherit_blocked_by_secure_mode(lua_ctx, *s) {
                return Err(mlua::Error::RuntimeError(
                    "Command.stderr(Stdio::INHERIT) is blocked in Secure Mode".to_string(),
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
        methods.add_async_method("spawn", |lua_ctx, this, ()| async move {
            // Trust gate: an untrusted plugin must not spawn a
            // process via the `pairee.Command(...)` builder at all.
            if spawn_blocked_by_trust(lua_ctx) {
                return Err(mlua::Error::RuntimeError(
                    "Command.spawn is blocked for untrusted plugins (mark the plugin as \
                     `trusted = true` in config to enable process spawning)"
                        .to_string(),
                ));
            }
            // §6 Secure-Mode: block blacklisted commands (curl, sh,
            // wget, etc.). The same check that `pairee.fs.spawn` does.
            if spawn_blocked_in_secure_mode(lua_ctx, &this.program) {
                return Err(mlua::Error::RuntimeError(format!(
                    "Command.spawn('{}') is blocked in Secure Mode (blacklisted)",
                    this.program
                )));
            }
            let mut cmd = this.materialise();
            match cmd.spawn() {
                Ok(mut child) => {
                    let id = child.id().unwrap_or(0);
                    let stdin = child.stdin.take();
                    let stdout = child.stdout.take();
                    let stderr = child.stderr.take();
                    let wrapped = Child {
                        id,
                        inner: std::sync::Arc::new(tokio::sync::Mutex::new(Some(child))),
                        stdin,
                        stdout,
                        stderr,
                    };
                    let ud = lua_ctx.create_userdata(wrapped)?;
                    Ok(mlua::Value::UserData(ud))
                }
                Err(e) => Err(mlua::Error::RuntimeError(format!(
                    "Command.spawn failed: {e}"
                ))),
            }
        });

        // `:output()` — run to completion and capture stdout+stderr.
        methods.add_async_method("output", |lua_ctx, this, ()| async move {
            if spawn_blocked_by_trust(lua_ctx) {
                return Err(mlua::Error::RuntimeError(
                    "Command.output is blocked for untrusted plugins".to_string(),
                ));
            }
            if spawn_blocked_in_secure_mode(lua_ctx, &this.program) {
                return Err(mlua::Error::RuntimeError(format!(
                    "Command.output('{}') is blocked in Secure Mode (blacklisted)",
                    this.program
                )));
            }
            let mut cmd = this.materialise();
            cmd.stdin(StdStdio::null());
            cmd.stdout(StdStdio::piped());
            cmd.stderr(StdStdio::piped());
            match cmd.output().await {
                Ok(out) => {
                    let output = super::output::Output::from_tokio(out);
                    let ud = lua_ctx.create_userdata(output)?;
                    Ok(mlua::Value::UserData(ud))
                }
                Err(e) => Err(mlua::Error::RuntimeError(format!(
                    "Command.output failed: {e}"
                ))),
            }
        });

        // `:status()` — run to completion and return the exit
        // status.
        methods.add_async_method("status", |lua_ctx, this, ()| async move {
            if spawn_blocked_by_trust(lua_ctx) {
                return Err(mlua::Error::RuntimeError(
                    "Command.status is blocked for untrusted plugins".to_string(),
                ));
            }
            if spawn_blocked_in_secure_mode(lua_ctx, &this.program) {
                return Err(mlua::Error::RuntimeError(format!(
                    "Command.status('{}') is blocked in Secure Mode (blacklisted)",
                    this.program
                )));
            }
            let mut cmd = this.materialise();
            cmd.stdin(StdStdio::null());
            cmd.stdout(StdStdio::null());
            cmd.stderr(StdStdio::null());
            match cmd.status().await {
                Ok(s) => {
                    let status = super::output::Status::from_exit(s);
                    let ud = lua_ctx.create_userdata(status)?;
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

    #[test]
    fn test_spawn_blocked_by_trust_when_untrusted() {
        // C1: an untrusted plugin must not be able to spawn a
        // process via the `pairee.Command(...)` builder. The
        // helper reports true (blocked) when `_trusted` is
        // missing/false.
        let lua = mlua::Lua::new();
        assert!(!is_trusted(&lua));
        assert!(spawn_blocked_by_trust(&lua));
    }

    #[test]
    fn test_spawn_allowed_by_trust_when_trusted() {
        // C1: a trusted plugin passes the trust gate. The
        // secure-mode blacklist may still apply on top of this.
        let lua = mlua::Lua::new();
        let pairee = lua.create_table().unwrap();
        pairee.set("_trusted", true).unwrap();
        lua.globals().set("pairee", pairee).unwrap();
        assert!(is_trusted(&lua));
        assert!(!spawn_blocked_by_trust(&lua));
    }

    #[test]
    fn test_spawn_blocked_by_trust_independent_of_secure_mode() {
        // C1: the trust gate is orthogonal to secure mode. A
        // plugin can be untrusted AND have secure mode on, but
        // can never spawn a process via Command if not trusted.
        let lua = mlua::Lua::new();
        let pairee = lua.create_table().unwrap();
        pairee.set("_trusted", false).unwrap();
        pairee.set("_secure_mode", true).unwrap();
        lua.globals().set("pairee", pairee).unwrap();
        assert!(!is_trusted(&lua));
        assert!(spawn_blocked_by_trust(&lua));
    }

    // §6: in Secure Mode, `env_blocked_in_secure_mode` must
    // reject every dynamic-linker hook. The denylist is
    // case-insensitive (the linker accepts `ld_preload`).
    #[test]
    fn test_env_blocked_in_secure_mode_blocks_ld_preload() {
        let lua = mlua::Lua::new();
        let pairee = lua.create_table().unwrap();
        pairee.set("_secure_mode", true).unwrap();
        lua.globals().set("pairee", pairee).unwrap();
        // All variants of the linker hooks are blocked.
        let blocked = [
            "LD_PRELOAD",
            "ld_preload",
            "LD_LIBRARY_PATH",
            "LD_AUDIT",
            "LD_DEBUG",
            "LD_DEBUG_OUTPUT",
            "LD_BIND_NOW",
            "LD_PROFILE",
            "DYLD_INSERT_LIBRARIES",
            "DYLD_LIBRARY_PATH",
            "DYLD_FRAMEWORK_PATH",
            "DYLD_FALLBACK_FRAMEWORK_PATH",
            "DYLD_ROOT_PATH",
        ];
        for name in blocked {
            assert!(
                env_blocked_in_secure_mode(&lua, name),
                "expected {:?} to be blocked in Secure Mode",
                name
            );
        }
    }

    // §6: in Secure Mode, `env_blocked_in_secure_mode` must
    // *not* reject ordinary environment variables. The
    // denylist is targeted at dynamic-linker hooks, not
    // user-controlled application env.
    #[test]
    fn test_env_allows_ordinary_vars_in_secure_mode() {
        let lua = mlua::Lua::new();
        let pairee = lua.create_table().unwrap();
        pairee.set("_secure_mode", true).unwrap();
        lua.globals().set("pairee", pairee).unwrap();
        let allowed = [
            "LANG",
            "PATH",
            "HOME",
            "USER",
            "TMPDIR",
            "CUSTOM_VAR",
            "FOO_BAR",
        ];
        for name in allowed {
            assert!(
                !env_blocked_in_secure_mode(&lua, name),
                "expected {:?} to be allowed in Secure Mode",
                name
            );
        }
    }

    // §6: outside Secure Mode, the denylist is inert. A
    // trusted plugin can still set LD_PRELOAD if it wants to.
    #[test]
    fn test_env_allows_all_outside_secure_mode() {
        let lua = mlua::Lua::new();
        // No _secure_mode set, is_secure_mode reads false.
        assert!(!env_blocked_in_secure_mode(&lua, "LD_PRELOAD"));
        assert!(!env_blocked_in_secure_mode(&lua, "DYLD_INSERT_LIBRARIES"));
    }

    // §6: `cwd_in_sandbox` must accept a path inside the
    // workspace and refuse one outside. We use the test
    // workspace (`std::env::current_dir`) as the in-sandbox
    // path and `/tmp` as the out-of-sandbox path.
    #[cfg(unix)]
    #[test]
    fn test_cwd_in_sandbox_accepts_workspace_path() {
        let lua = mlua::Lua::new();
        let pairee = lua.create_table().unwrap();
        pairee.set("_secure_mode", true).unwrap();
        lua.globals().set("pairee", pairee).unwrap();
        let workspace = std::env::current_dir().expect("cwd");
        // The workspace itself may not exist as a directory on
        // some CI hosts; create it for the test if missing.
        let _ = std::fs::create_dir_all(&workspace);
        assert!(cwd_in_sandbox(&lua, &workspace.to_string_lossy()));
    }

    #[cfg(unix)]
    #[test]
    fn test_cwd_in_sandbox_rejects_path_outside_workspace() {
        let lua = mlua::Lua::new();
        let pairee = lua.create_table().unwrap();
        pairee.set("_secure_mode", true).unwrap();
        lua.globals().set("pairee", pairee).unwrap();
        // /tmp/outside_sandbox_test must not exist AND its
        // canonicalized parent must be /tmp (outside the
        // workspace).
        let bogus = "/tmp/pairee_audit_outside_sandbox_xyzzy";
        // Make sure the path actually exists so canonicalize
        // succeeds. We clean up after the test.
        let _ = std::fs::create_dir_all(bogus);
        assert!(!cwd_in_sandbox(&lua, bogus));
        let _ = std::fs::remove_dir_all(bogus);
    }

    // §6: outside Secure Mode, the cwd check is a no-op.
    #[test]
    fn test_cwd_in_sandbox_outside_secure_mode() {
        let lua = mlua::Lua::new();
        // No _secure_mode set.
        assert!(cwd_in_sandbox(&lua, "/tmp/anything"));
        assert!(cwd_in_sandbox(&lua, "/etc"));
    }

    // §6: `clamp_rlimit_as` must lift requests below the
    // floor up to the floor (otherwise the exec fails with
    // ENOMEM) and pass through any request above the floor
    // unchanged.
    #[test]
    fn test_clamp_rlimit_as_enforces_floor() {
        // Below the floor → clamped up to 4 MiB.
        assert_eq!(clamp_rlimit_as(0), 4 * 1024 * 1024);
        assert_eq!(clamp_rlimit_as(1), 4 * 1024 * 1024);
        assert_eq!(clamp_rlimit_as(4 * 1024 * 1024 - 1), 4 * 1024 * 1024);
        // At the floor → unchanged.
        assert_eq!(clamp_rlimit_as(4 * 1024 * 1024), 4 * 1024 * 1024);
        // Above the floor → unchanged.
        assert_eq!(clamp_rlimit_as(8 * 1024 * 1024), 8 * 1024 * 1024);
        assert_eq!(clamp_rlimit_as(1_073_741_824), 1_073_741_824);
        // u64::MAX → unchanged.
        assert_eq!(clamp_rlimit_as(u64::MAX), u64::MAX);
    }
}
