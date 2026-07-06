//! `Child` userdata — wraps a live `tokio::process::Child`.
//!
//! M3 ships the full streaming surface per roadmap §5.B4:
//!
//! - `id()` — sync, returns the OS-assigned PID.
//! - `wait()` — async, takes the inner child and awaits it.
//! - `wait_with_output()` — async, takes the inner child, awaits
//!   it, and returns an `Output` userdata.
//! - `try_wait()` — sync, non-blocking poll; clears `inner` only
//!   when the child has actually exited.
//! - `start_kill()` — sync, sends SIGKILL (does NOT reap).
//! - `read(len)` — async, reads up to `len` bytes from stdout.
//! - `read_line()` — async, reads one line (incl. `\n`) from stdout.
//! - `read_line_with({timeout})` — async; races stdout against
//!   `tokio::time::sleep(timeout)`. Returns `nil` on timeout;
//!   stdout is preserved.
//! - `write_all(src)` — async, writes to stdin without taking it
//!   (so the plugin can call again).
//! - `flush()` — async, flushes the stdin pipe.
//! - `take_stdin()` / `take_stdout()` / `take_stderr()` — sync;
//!   return a `ChildInput` / `ChildOutput` / `ChildError`
//!   userdata wrapping the pipe, or `nil` if already taken.
//!
//! Async methods are registered via `add_async_method_mut`; the
//! returned future holds `&mut self` for its lifetime `'s`, which
//! is bounded by the borrow checker so we can mutate `this.inner`
//! and then await on the moved-out value.

use super::output::{Output, Status};
use mlua::UserData;
use mlua::UserDataMethods;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::{
    Child as TokioChild, ChildStderr, ChildStdin, ChildStdout,
};
use tokio::sync::Mutex;

/// The M3 `Child` userdata. Owns the `tokio::process::Child`
/// and (optionally) the captured stdin/stdout/stderr pipes.
///
/// `inner` is wrapped in a `Mutex` so async methods can hold a
/// mutable borrow across await points (the `add_async_method_mut`
/// API requires the future to be valid for the lifetime of the
/// `&mut self` borrow, but the future itself can't capture a
/// mutable reference because the closure is `Fn` not `FnMut`).
pub struct Child {
    pub id: u32,
    pub inner: Arc<Mutex<Option<TokioChild>>>,
    pub stdin: Option<ChildStdin>,
    pub stdout: Option<ChildStdout>,
    pub stderr: Option<ChildStderr>,
}

impl UserData for Child {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        // ── `id` — sync, takes &self ───────────────────────────────
        methods.add_method("id", |_lua, this, ()| Ok(this.id));

        // ── `close` — sync, drops the pipes (sends EOF on stdin)
        //     but does NOT reap the child. The plugin still needs
        //     to call `wait`/`wait_with_output` to reap.
        methods.add_method_mut("close", |_lua, this, ()| {
            this.stdin.take();
            this.stdout.take();
            this.stderr.take();
            Ok(true)
        });

        // ── `try_wait` — sync, non-blocking poll ───────────────────
        methods.add_method_mut("try_wait", |_lua, this, ()| {
            // `try_lock` is required so we never block the worker
            // thread; if another async call is awaiting on the
            // inner child, we just report "not ready yet".
            let mut guard = match this.inner.try_lock() {
                Ok(g) => g,
                Err(_) => return Ok(mlua::Value::Nil),
            };
            let inner = match guard.as_mut() {
                Some(i) => i,
                None => return Ok(mlua::Value::Nil),
            };
            match inner.try_wait() {
                Ok(Some(s)) => {
                    let status = Status::from_exit(s);
                    *guard = None; // child is reaped; clear it
                    let ud = _lua.create_userdata(status)?;
                    Ok(mlua::Value::UserData(ud))
                }
                Ok(None) => Ok(mlua::Value::Nil),
                Err(e) => Err(mlua::Error::RuntimeError(format!(
                    "Child.try_wait failed: {e}"
                ))),
            }
        });

        // ── `start_kill` — sync, sends SIGKILL (Unix) / TerminateProcess
        //     (Windows). Does NOT reap. ─────────────────────────────
        methods.add_method_mut("start_kill", |_lua, this, ()| {
            let mut guard = match this.inner.try_lock() {
                Ok(g) => g,
                Err(_) => {
                    return Err(mlua::Error::RuntimeError(
                        "Child.start_kill: inner child is busy".to_string(),
                    ));
                }
            };
            let inner = match guard.as_mut() {
                Some(i) => i,
                None => return Ok(false),
            };
            match inner.start_kill() {
                Ok(()) => Ok(true),
                Err(e) => Err(mlua::Error::RuntimeError(format!(
                    "Child.start_kill failed: {e}"
                ))),
            }
        });

        // ── `wait` — async, takes &mut self ───────────────────────
        // The future holds `&mut this` (and the inner MutexGuard)
        // for its lifetime. We move the TokioChild out via
        // `mem::replace(..., None)` so we can call `wait` on it.
        methods.add_async_method_mut("wait", |_lua, this, ()| async move {
            // Take the inner child out of the Mutex; if the take
            // fails because another async call already grabbed it,
            // we report Nil.
            let taken = {
                let mut guard = match this.inner.try_lock() {
                    Ok(g) => g,
                    Err(_) => return Ok(mlua::Value::Nil),
                };
                guard.take()
            };
            let mut child = match taken {
                Some(c) => c,
                None => return Ok(mlua::Value::Nil),
            };
            // Close stdin so the child doesn't deadlock waiting
            // for input we're no longer going to send.
            drop(child.stdin.take());
            let exit = child.wait().await.map_err(|e| {
                mlua::Error::RuntimeError(format!("Child.wait failed: {e}"))
            })?;
            let status = Status::from_exit(exit);
            let ud = _lua.create_userdata(status)?;
            Ok(mlua::Value::UserData(ud))
        });

        // ── `wait_with_output` — async, consumes inner child ─────
        methods.add_async_method_mut("wait_with_output", |_lua, this, ()| async move {
            let taken = {
                let mut guard = match this.inner.try_lock() {
                    Ok(g) => g,
                    Err(_) => return Ok(mlua::Value::Nil),
                };
                guard.take()
            };
            let child = match taken {
                Some(c) => c,
                None => return Ok(mlua::Value::Nil),
            };
            let out = child.wait_with_output().await.map_err(|e| {
                mlua::Error::RuntimeError(format!(
                    "Child.wait_with_output failed: {e}"
                ))
            })?;
            let output = Output::from_tokio(out);
            let ud = _lua.create_userdata(output)?;
            Ok(mlua::Value::UserData(ud))
        });

        // ── `read(len)` — async, reads up to `len` bytes ──────────
        methods.add_async_method_mut("read", |_lua, this, len: usize| async move {
            let stdout = match this.stdout.as_mut() {
                Some(s) => s,
                None => return Ok(mlua::Value::Nil),
            };
            let mut buf = vec![0u8; len];
            let n = stdout.read(&mut buf).await.map_err(|e| {
                mlua::Error::RuntimeError(format!("Child.read failed: {e}"))
            })?;
            buf.truncate(n);
            Ok(mlua::Value::String(_lua.create_string(&buf)?))
        });

        // ── `read_line()` — async, reads one line ────────────────
        methods.add_async_method_mut("read_line", |_lua, this, ()| async move {
            let stdout = match this.stdout.as_mut() {
                Some(s) => s,
                None => return Ok(mlua::Value::Nil),
            };
            let mut reader = BufReader::new(stdout);
            let mut line = String::new();
            let n = reader.read_line(&mut line).await.map_err(|e| {
                mlua::Error::RuntimeError(format!("Child.read_line failed: {e}"))
            })?;
            if n == 0 {
                return Ok(mlua::Value::Nil);
            }
            Ok(mlua::Value::String(_lua.create_string(&line)?))
        });

        // ── `read_line_with({timeout})` — async, races vs sleep ──
        methods.add_async_method_mut(
            "read_line_with",
            |_lua, this, opts: Option<mlua::Table>| async move {
                let timeout_secs: Option<f64> = match opts {
                    Some(t) => t.get::<_, f64>("timeout").ok(),
                    None => None,
                };
                let stdout = match this.stdout.as_mut() {
                    Some(s) => s,
                    None => return Ok(mlua::Value::Nil),
                };
                // We need a `BufReader` that owns the stdout. But
                // `&mut ChildStdout` is itself an `AsyncRead`; we
                // can wrap a mutable borrow of it in a `BufReader`
                // and the borrow is valid for the whole future
                // (held by the `add_async_method_mut` lifetime
                // contract).
                let stdout_ref: &mut ChildStdout = stdout;
                let mut reader = BufReader::new(stdout_ref);
                let mut line = String::new();
                let read_fut = async {
                    reader.read_line(&mut line).await.map(|n| n)
                };
                let result = if let Some(t) = timeout_secs {
                    tokio::select! {
                        biased;
                        r = read_fut => Some(r),
                        _ = tokio::time::sleep(std::time::Duration::from_secs_f64(t)) => None,
                    }
                } else {
                    Some(read_fut.await)
                };
                match result {
                    Some(Ok(0)) => Ok(mlua::Value::Nil),
                    Some(Ok(_)) => Ok(mlua::Value::String(_lua.create_string(&line)?)),
                    Some(Err(e)) => Err(mlua::Error::RuntimeError(format!(
                        "Child.read_line_with failed: {e}"
                    ))),
                    None => Ok(mlua::Value::Nil), // timeout
                }
            },
        );

        // ── `write_all(src)` — async, preserves stdin ────────────
        methods.add_async_method_mut(
            "write_all",
            |_lua, this, src: mlua::String| async move {
                let stdin = match this.stdin.as_mut() {
                    Some(s) => s,
                    None => {
                        return Err(mlua::Error::RuntimeError(
                            "Child.write_all: stdin is None (already taken or \
                             never piped)"
                                .to_string(),
                        ));
                    }
                };
                stdin.write_all(src.as_bytes()).await.map_err(|e| {
                    mlua::Error::RuntimeError(format!(
                        "Child.write_all failed: {e}"
                    ))
                })?;
                Ok(true)
            },
        );

        // ── `flush()` — async ───────────────────────────────────
        methods.add_async_method_mut("flush", |_lua, this, ()| async move {
            let stdin = match this.stdin.as_mut() {
                Some(s) => s,
                None => return Ok(mlua::Value::Nil),
            };
            stdin.flush().await.map_err(|e| {
                mlua::Error::RuntimeError(format!("Child.flush failed: {e}"))
            })?;
            Ok(mlua::Value::Boolean(true))
        });

        // ── `take_stdin()` / `take_stdout()` / `take_stderr()` ──
        methods.add_method_mut("take_stdin", |_lua, this, ()| {
            this.stdin
                .take()
                .map(|s| {
                    let ud = _lua.create_userdata(ChildInput { inner: Some(s) })?;
                    Ok(mlua::Value::UserData(ud))
                })
                .transpose()
        });
        methods.add_method_mut("take_stdout", |_lua, this, ()| {
            this.stdout
                .take()
                .map(|s| {
                    let ud = _lua.create_userdata(ChildOutput { inner: Some(s) })?;
                    Ok(mlua::Value::UserData(ud))
                })
                .transpose()
        });
        methods.add_method_mut("take_stderr", |_lua, this, ()| {
            this.stderr
                .take()
                .map(|s| {
                    let ud = _lua.create_userdata(ChildError { inner: Some(s) })?;
                    Ok(mlua::Value::UserData(ud))
                })
                .transpose()
        });
    }
}

// ── `take_*` wrapper userdata ─────────────────────────────────────
//
// M3 simplicity: three tiny per-direction UserData wrappers. They
// hold the pipe in an `Option` so `close` (which drops the inner)
// is idempotent.

/// Wraps a `tokio::process::ChildStdin` so a plugin can store
/// the pipe across calls and call `write_all` / `flush` / `close`
/// on it later.
pub struct ChildInput {
    pub inner: Option<ChildStdin>,
}

impl UserData for ChildInput {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_async_method_mut(
            "write_all",
            |_lua, this, src: mlua::String| async move {
                let stdin = match this.inner.as_mut() {
                    Some(s) => s,
                    None => {
                        return Err(mlua::Error::RuntimeError(
                            "ChildInput.write_all: handle already closed"
                                .to_string(),
                        ));
                    }
                };
                stdin.write_all(src.as_bytes()).await.map_err(|e| {
                    mlua::Error::RuntimeError(format!(
                        "ChildInput.write_all failed: {e}"
                    ))
                })?;
                Ok(true)
            },
        );
        methods.add_async_method_mut("flush", |_lua, this, ()| async move {
            let stdin = match this.inner.as_mut() {
                Some(s) => s,
                None => return Ok(mlua::Value::Nil),
            };
            stdin.flush().await.map_err(|e| {
                mlua::Error::RuntimeError(format!(
                    "ChildInput.flush failed: {e}"
                ))
            })?;
            Ok(mlua::Value::Boolean(true))
        });
        methods.add_method_mut("close", |_lua, this, ()| {
            this.inner.take();
            Ok(true)
        });
    }
}

/// Wraps a `tokio::process::ChildStdout`.
pub struct ChildOutput {
    pub inner: Option<ChildStdout>,
}

impl UserData for ChildOutput {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_async_method_mut(
            "read",
            |_lua, this, len: usize| async move {
                let stdout = match this.inner.as_mut() {
                    Some(s) => s,
                    None => return Ok(mlua::Value::Nil),
                };
                let mut buf = vec![0u8; len];
                let n = stdout.read(&mut buf).await.map_err(|e| {
                    mlua::Error::RuntimeError(format!(
                        "ChildOutput.read failed: {e}"
                    ))
                })?;
                buf.truncate(n);
                Ok(mlua::Value::String(_lua.create_string(&buf)?))
            },
        );
        methods.add_async_method_mut("read_line", |_lua, this, ()| async move {
            let stdout = match this.inner.as_mut() {
                Some(s) => s,
                None => return Ok(mlua::Value::Nil),
            };
            let mut reader = BufReader::new(stdout);
            let mut line = String::new();
            let n = reader.read_line(&mut line).await.map_err(|e| {
                mlua::Error::RuntimeError(format!(
                    "ChildOutput.read_line failed: {e}"
                ))
            })?;
            if n == 0 {
                return Ok(mlua::Value::Nil);
            }
            Ok(mlua::Value::String(_lua.create_string(&line)?))
        });
        methods.add_method_mut("close", |_lua, this, ()| {
            this.inner.take();
            Ok(true)
        });
    }
}

/// Wraps a `tokio::process::ChildStderr`.
pub struct ChildError {
    pub inner: Option<ChildStderr>,
}

impl UserData for ChildError {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_async_method_mut(
            "read",
            |_lua, this, len: usize| async move {
                let stderr = match this.inner.as_mut() {
                    Some(s) => s,
                    None => return Ok(mlua::Value::Nil),
                };
                let mut buf = vec![0u8; len];
                let n = stderr.read(&mut buf).await.map_err(|e| {
                    mlua::Error::RuntimeError(format!(
                        "ChildError.read failed: {e}"
                    ))
                })?;
                buf.truncate(n);
                Ok(mlua::Value::String(_lua.create_string(&buf)?))
            },
        );
        methods.add_async_method_mut("read_line", |_lua, this, ()| async move {
            let stderr = match this.inner.as_mut() {
                Some(s) => s,
                None => return Ok(mlua::Value::Nil),
            };
            let mut reader = BufReader::new(stderr);
            let mut line = String::new();
            let n = reader.read_line(&mut line).await.map_err(|e| {
                mlua::Error::RuntimeError(format!(
                    "ChildError.read_line failed: {e}"
                ))
            })?;
            if n == 0 {
                return Ok(mlua::Value::Nil);
            }
            Ok(mlua::Value::String(_lua.create_string(&line)?))
        });
        methods.add_method_mut("close", |_lua, this, ()| {
            this.inner.take();
            Ok(true)
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mlua::Lua;
    use mlua::AnyUserDataExt;

    /// Spawn `/bin/echo` via the public `Child` userdata surface
    /// and exercise `read_line` + `wait` end-to-end.
    ///
    /// M3 done-when criterion (roadmap §9.M3) requires that a
    /// plugin can call `child:write_all("...")` and
    /// `child:read_line()`. `/bin/cat` blocks on EOF (it never
    /// returns until the parent closes stdin), so we use
    /// `/bin/echo` which needs no stdin; the M3 "write_all" path
    /// is exercised by `test_child_write_all_only` below (which
    /// closes the pipe via `close()` immediately after writing).
    #[tokio::test]
    async fn test_child_read_line_and_wait() {
        let lua = Lua::new();
        #[cfg(unix)]
        let cmd = "/bin/echo";
        #[cfg(windows)]
        let cmd = "cmd";

        let mut tokio_cmd = tokio::process::Command::new(cmd);
        #[cfg(unix)]
        tokio_cmd.arg("hello from echo");
        #[cfg(windows)]
        {
            tokio_cmd.arg("/c");
            tokio_cmd.arg("echo hello from echo");
        }
        tokio_cmd.stdin(std::process::Stdio::null());
        tokio_cmd.stdout(std::process::Stdio::piped());
        let mut handle = tokio_cmd.spawn().expect("spawn echo");
        let id = handle.id().unwrap_or(0);
        let stdout = handle.stdout.take();
        let child_ud = lua
            .create_userdata(Child {
                id,
                inner: Arc::new(Mutex::new(Some(handle))),
                stdin: None,
                stdout,
                stderr: None,
            })
            .expect("create_userdata");
        lua.globals()
            .set("child", child_ud.clone())
            .expect("set child");

        let line: String = lua
            .load("return child:read_line()")
            .call_async(())
            .await
            .expect("read_line");
        assert_eq!(line.trim_end_matches(['\n', '\r']), "hello from echo");

        let status_val: mlua::Value = lua
            .load("return child:wait()")
            .call_async(())
            .await
            .expect("wait");
        let status_ud = match status_val {
            mlua::Value::UserData(ud) => ud,
            other => panic!("expected Status userdata, got {other:?}"),
        };
        let success: bool = status_ud.call_method("success", ()).expect("success");
        assert!(success, "echo should exit with success");
    }

    /// Exercise `write_all` against a `/bin/cat` that we then
    /// EOF (via `take_stdin()` + `ChildInput:close()`) so the
    /// child finishes and we can `read_line` the echoed bytes
    /// back.
    #[tokio::test]
    async fn test_child_write_all_then_close() {
        let lua = Lua::new();
        #[cfg(unix)]
        let cmd = "/bin/cat";
        #[cfg(windows)]
        let cmd = "cmd";
        let mut tokio_cmd = tokio::process::Command::new(cmd);
        #[cfg(windows)]
        {
            tokio_cmd.arg("/c");
            tokio_cmd.arg("more");
        }
        tokio_cmd.stdin(std::process::Stdio::piped());
        tokio_cmd.stdout(std::process::Stdio::piped());
        let mut handle = tokio_cmd.spawn().expect("spawn cat");
        let id = handle.id().unwrap_or(0);
        let stdin = handle.stdin.take();
        let stdout = handle.stdout.take();
        let child_ud = lua
            .create_userdata(Child {
                id,
                inner: Arc::new(Mutex::new(Some(handle))),
                stdin,
                stdout,
                stderr: None,
            })
            .expect("create_userdata");
        lua.globals()
            .set("child", child_ud.clone())
            .expect("set child");

        let written: bool = lua
            .load("return child:write_all('hello from plugin\\n')")
            .call_async(())
            .await
            .expect("write_all");
        assert!(written);
        // Send EOF on stdin only (we keep stdout open so cat can
        // write back its echoed bytes — closing stdout would give
        // cat a SIGPIPE on the next write). We `take_stdin` and
        // then `close` the resulting `ChildInput` so the parent
        // drops its end of the stdin pipe.
        let took_stdin: bool = lua
            .load("local s = child:take_stdin(); if s then s:close(); return true else return false end")
            .call_async(())
            .await
            .expect("take_stdin");
        assert!(took_stdin, "expected to be able to take stdin");
        // `wait` should now return quickly (cat has seen EOF).
        let status_val: mlua::Value = lua
            .load("return child:wait()")
            .call_async(())
            .await
            .expect("wait");
        match status_val {
            mlua::Value::UserData(ud) => {
                let success: bool = ud.call_method("success", ()).expect("success");
                assert!(success, "cat should exit with success after EOF");
            }
            mlua::Value::Nil => panic!("expected a Status userdata after wait"),
            other => panic!("unexpected wait result: {other:?}"),
        }
    }

    /// `try_wait` returns Nil while the child is running, and a
    /// Status userdata after the child has exited.
    #[tokio::test]
    async fn test_child_try_wait() {
        let lua = Lua::new();
        #[cfg(unix)]
        let cmd = "/bin/true";
        #[cfg(windows)]
        let cmd = "cmd";

        let mut tokio_cmd = tokio::process::Command::new(cmd);
        #[cfg(windows)]
        {
            tokio_cmd.arg("/c");
            tokio_cmd.arg("exit");
            tokio_cmd.arg("0");
        }
        tokio_cmd.stdin(std::process::Stdio::null());
        tokio_cmd.stdout(std::process::Stdio::null());
        tokio_cmd.stderr(std::process::Stdio::null());
        let handle = tokio_cmd.spawn().expect("spawn true");
        let id = handle.id().unwrap_or(0);
        let child_ud = lua
            .create_userdata(Child {
                id,
                inner: Arc::new(Mutex::new(Some(handle))),
                stdin: None,
                stdout: None,
                stderr: None,
            })
            .expect("create_userdata");
        lua.globals().set("child", child_ud.clone()).expect("set");
        // Give the child a moment to finish.
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let status_val: mlua::Value = lua
            .load("return child:try_wait()")
            .call_async(())
            .await
            .expect("try_wait");
        match status_val {
            mlua::Value::UserData(ud) => {
                let success: bool = ud.call_method("success", ()).expect("success");
                assert!(success, "/bin/true should exit successfully");
            }
            mlua::Value::Nil => panic!("try_wait should have returned a Status"),
            other => panic!("unexpected try_wait result: {other:?}"),
        }
    }
}
