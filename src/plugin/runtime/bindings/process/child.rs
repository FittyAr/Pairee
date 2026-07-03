//! `Child` userdata — wraps a live `tokio::process::Child`.
//!
//! M3 only exposes the high-level accessors (id, wait, kill,
//! wait_with_output) because the streaming read/write API
//! requires `take_stdin`/`take_stdout`/`take_stderr` which
//! need careful lifetime management. The roadmap promises
//! the full surface in M3; for now the streaming read/write
//! methods are stubs that return an error.

use super::output::{Output, Status};
use mlua::{UserData, UserDataMethods};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child as TokioChild, ChildStderr, ChildStdin, ChildStdout};

/// The M3 `Child` userdata. Owns the `tokio::process::Child`
/// and (optionally) the captured stdin/stdout/stderr pipes.
pub struct Child {
    pub id: u32,
    /// `Some` while the child is running; set to `None` after
    /// `wait()`/`wait_with_output()` consume the inner handle.
    pub inner: Option<TokioChild>,
    pub stdin: Option<ChildStdin>,
    pub stdout: Option<ChildStdout>,
    pub stderr: Option<ChildStderr>,
}

impl UserData for Child {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method("id", |_lua, this, ()| Ok(this.id));
        methods.add_async_method("wait", |_lua, this, ()| async move {
            if let Some(mut inner) = this.inner.take() {
                match inner.wait().await {
                    Ok(s) => {
                        let status = Status::from_exit(s);
                        let ud = _lua.create_userdata(status)?;
                        Ok(mlua::Value::UserData(ud))
                    }
                    Err(e) => Err(mlua::Error::RuntimeError(format!(
                        "Child.wait failed: {e}"
                    ))),
                }
            } else {
                Err(mlua::Error::RuntimeError(
                    "Child.wait: child already consumed".to_string(),
                ))
            }
        });
        methods.add_async_method("try_wait", |_lua, this, ()| async move {
            // `try_wait` is sync but the async wrapper awaits on
            // an Immediate future. We just call it directly.
            if let Some(inner) = this.inner.as_ref() {
                match inner.try_wait() {
                    Ok(Some(s)) => {
                        let status = Status::from_exit(s);
                        let ud = _lua.create_userdata(status)?;
                        Ok(mlua::Value::UserData(ud))
                    }
                    Ok(None) => Ok(mlua::Value::Nil),
                    Err(e) => Err(mlua::Error::RuntimeError(format!(
                        "Child.try_wait failed: {e}"
                    ))),
                }
            } else {
                Ok(mlua::Value::Nil)
            }
        });
        methods.add_async_method("wait_with_output", |_lua, mut this, ()| async move {
            // We need to consume the inner child. `output()`
            // requires piped stdio; if the caller did not set
            // `Stdio::Piped` for stdout/stderr, we get empty
            // strings.
            let inner = match this.inner.take() {
                Some(c) => c,
                None => {
                    return Err(mlua::Error::RuntimeError(
                        "Child.wait_with_output: child already consumed".to_string(),
                    ))
                }
            };
            match inner.wait_with_output().await {
                Ok(out) => {
                    let output = Output::from_tokio(out);
                    let ud = _lua.create_userdata(output)?;
                    Ok(mlua::Value::UserData(ud))
                }
                Err(e) => Err(mlua::Error::RuntimeError(format!(
                    "Child.wait_with_output failed: {e}"
                ))),
            }
        });
        methods.add_async_method("start_kill", |_lua, this, ()| async move {
            if let Some(mut inner) = this.inner.take() {
                match inner.start_kill() {
                    Ok(()) => Ok(true),
                    Err(e) => Err(mlua::Error::RuntimeError(format!(
                        "Child.start_kill failed: {e}"
                    ))),
                }
            } else {
                Ok(false)
            }
        });
        // Streaming read — supported in M3 for stdout/stderr
        // when piped.
        methods.add_async_method("read", |_lua, this, len: usize| async move {
            let stdout = this.stdout.take();
            match stdout {
                Some(stdout) => {
                    let mut reader = BufReader::new(stdout);
                    let mut buf = vec![0u8; len];
                    let n = match reader.read(&mut buf).await {
                        Ok(n) => n,
                        Err(e) => {
                            return Err(mlua::Error::RuntimeError(format!(
                                "Child.read failed: {e}"
                            )));
                        }
                    };
                    buf.truncate(n);
                    Ok(mlua::Value::String(_lua.create_string(&buf)?))
                }
                None => Err(mlua::Error::RuntimeError(
                    "Child.read: stdout is not piped".to_string(),
                )),
            }
        });
        methods.add_async_method("read_line", |_lua, this, ()| async move {
            let stdout = this.stdout.take();
            match stdout {
                Some(stdout) => {
                    let mut reader = BufReader::new(stdout);
                    let mut line = String::new();
                    let n = match reader.read_line(&mut line).await {
                        Ok(n) => n,
                        Err(e) => {
                            return Err(mlua::Error::RuntimeError(format!(
                                "Child.read_line failed: {e}"
                            )));
                        }
                    };
                    if n == 0 {
                        Ok(mlua::Value::Nil)
                    } else {
                        Ok(mlua::Value::String(_lua.create_string(&line)?))
                    }
                }
                None => Err(mlua::Error::RuntimeError(
                    "Child.read_line: stdout is not piped".to_string(),
                )),
            }
        });
        methods.add_async_method("write_all", |_lua, mut this, src: mlua::String| async move {
            let stdin = this.stdin.take();
            match stdin {
                Some(mut stdin) => {
                    stdin
                        .write_all(src.as_bytes())
                        .await
                        .map_err(|e| mlua::Error::RuntimeError(format!(
                            "Child.write_all failed: {e}"
                        )))?;
                    stdin
                        .flush()
                        .await
                        .map_err(|e| mlua::Error::RuntimeError(format!(
                            "Child.flush failed: {e}"
                        )))?;
                    Ok(true)
                }
                None => Err(mlua::Error::RuntimeError(
                    "Child.write_all: stdin is not piped".to_string(),
                )),
            }
        });
    }
}
