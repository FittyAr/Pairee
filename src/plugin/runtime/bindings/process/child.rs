//! Streaming `Child` userdata from `Command:spawn()`.

use super::capture::{self, OUTPUT_LIMIT};
use super::output::{LuaOutput, LuaStatus};
use mlua::{UserData, UserDataMethods};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// Most bytes one `read` / `read_line` call returns.
const MAX_READ: usize = 1024 * 1024;

/// Removes and returns up to `max` bytes of `buf` as text, cut at a
/// character boundary. `None` when `buf` holds only the start of one
/// character (more bytes are needed). Invalid bytes become U+FFFD.
fn split_text(buf: &mut Vec<u8>, max: usize) -> Option<String> {
    if buf.is_empty() {
        return Some(String::new());
    }
    let take = buf.len().min(max);
    let cut = match std::str::from_utf8(&buf[..take]) {
        Ok(_) => take,
        // Incomplete character at the end of the window.
        Err(e) if e.error_len().is_none() => match e.valid_up_to() {
            0 if take == buf.len() => return None,
            // The window is smaller than the character: cut anyway.
            0 => take,
            valid => valid,
        },
        Err(_) => take,
    };
    let rest = buf.split_off(cut);
    let head = std::mem::replace(buf, rest);
    Some(String::from_utf8_lossy(&head).into_owned())
}

pub struct LuaChild {
    pub child: Option<tokio::process::Child>,
    pub stdin: Option<tokio::process::ChildStdin>,
    pub stdout: Option<tokio::process::ChildStdout>,
    pub stderr: Option<tokio::process::ChildStderr>,
    pub leftover: Vec<u8>,
}

impl LuaChild {
    pub fn id(&self) -> Option<u32> {
        self.child.as_ref().and_then(|c| c.id())
    }

    pub fn start_kill(&mut self) -> mlua::Result<()> {
        if let Some(child) = self.child.as_mut() {
            child
                .start_kill()
                .map_err(|e| mlua::Error::RuntimeError(format!("kill failed: {e}")))?;
        }
        Ok(())
    }

    pub fn try_wait(&mut self) -> mlua::Result<Option<LuaStatus>> {
        let Some(child) = self.child.as_mut() else {
            return Ok(None);
        };
        match child.try_wait() {
            Ok(Some(status)) => Ok(Some(LuaStatus::from_exit(status))),
            Ok(None) => Ok(None),
            Err(e) => Err(mlua::Error::RuntimeError(format!("try_wait failed: {e}"))),
        }
    }

    pub async fn write_all(&mut self, src: &[u8]) -> mlua::Result<()> {
        let stdin = self.stdin.as_mut().ok_or_else(|| {
            mlua::Error::RuntimeError("child stdin is not piped or already closed".into())
        })?;
        stdin
            .write_all(src)
            .await
            .map_err(|e| mlua::Error::RuntimeError(format!("write failed: {e}")))?;
        stdin
            .flush()
            .await
            .map_err(|e| mlua::Error::RuntimeError(format!("flush failed: {e}")))
    }

    pub fn close_stdin(&mut self) {
        self.stdin = None;
    }

    /// Reads one chunk from stdout into `leftover`; `false` at end of file.
    async fn fill(&mut self) -> mlua::Result<bool> {
        let stdout = self
            .stdout
            .as_mut()
            .ok_or_else(|| mlua::Error::RuntimeError("child stdout is not piped".into()))?;
        let mut buf = [0u8; 4096];
        let n = stdout
            .read(&mut buf)
            .await
            .map_err(|e| mlua::Error::RuntimeError(format!("read failed: {e}")))?;
        self.leftover.extend_from_slice(&buf[..n]);
        Ok(n > 0)
    }

    /// Up to `len` bytes of text (at most [`MAX_READ`]), after what
    /// `read_line` left buffered; never splits a UTF-8 character.
    pub async fn read(&mut self, len: usize) -> mlua::Result<String> {
        let len = len.clamp(1, MAX_READ);
        if self.leftover.is_empty() {
            self.fill().await?;
        }
        loop {
            if let Some(text) = split_text(&mut self.leftover, len) {
                return Ok(text);
            }
            // Only the start of a multi-byte character is buffered.
            if !self.fill().await? {
                return Ok(
                    String::from_utf8_lossy(&std::mem::take(&mut self.leftover)).into_owned()
                );
            }
        }
    }

    /// The next line (with its `\n`); the rest of the data stays buffered.
    pub async fn read_line(&mut self) -> mlua::Result<String> {
        loop {
            if let Some(pos) = self.leftover.iter().position(|&b| b == b'\n') {
                let rest = self.leftover.split_off(pos + 1);
                let line = std::mem::replace(&mut self.leftover, rest);
                return Ok(String::from_utf8_lossy(&line).into_owned());
            }
            if self.leftover.len() >= MAX_READ || self.stdout.is_none() || !self.fill().await? {
                return Ok(
                    String::from_utf8_lossy(&std::mem::take(&mut self.leftover)).into_owned()
                );
            }
        }
    }

    pub async fn wait(&mut self) -> mlua::Result<LuaStatus> {
        let mut child = self
            .child
            .take()
            .ok_or_else(|| mlua::Error::RuntimeError("child already consumed".into()))?;
        let status = child
            .wait()
            .await
            .map_err(|e| mlua::Error::RuntimeError(format!("wait failed: {e}")))?;
        Ok(LuaStatus::from_exit(status))
    }

    pub async fn wait_with_output(&mut self) -> mlua::Result<LuaOutput> {
        let mut stdout = std::mem::take(&mut self.leftover);
        let rest = capture::read_limited(
            self.stdout.take(),
            OUTPUT_LIMIT.saturating_sub(stdout.len()),
        );
        let (rest, stderr) = tokio::try_join!(
            rest,
            capture::read_limited(self.stderr.take(), OUTPUT_LIMIT)
        )
        .map_err(|e| {
            let _ = self.start_kill();
            mlua::Error::RuntimeError(format!("read output failed: {e}"))
        })?;
        stdout.extend_from_slice(&rest);
        let status = self.wait().await?;
        Ok(LuaOutput {
            status,
            stdout: String::from_utf8_lossy(&stdout).into_owned(),
            stderr: String::from_utf8_lossy(&stderr).into_owned(),
        })
    }
}

impl UserData for LuaChild {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("id", |_, this, ()| Ok(this.id()));
        methods.add_method_mut("start_kill", |_, this, ()| this.start_kill());
        methods.add_method_mut("try_wait", |_, this, ()| this.try_wait());
        methods.add_method_mut("close_stdin", |_, this, ()| {
            this.close_stdin();
            Ok(())
        });

        methods.add_async_method_mut("write_all", |_, mut this, src: String| async move {
            this.write_all(src.as_bytes()).await
        });
        methods.add_async_method_mut("read", |_, mut this, len: Option<usize>| async move {
            this.read(len.unwrap_or(4096)).await
        });
        methods.add_async_method_mut("read_line", |_, mut this, ()| async move {
            this.read_line().await
        });
        methods.add_async_method_mut("wait", |_, mut this, ()| async move { this.wait().await });
        methods.add_async_method_mut("wait_with_output", |_, mut this, ()| async move {
            this.wait_with_output().await
        });
    }
}

#[cfg(test)]
mod tests {
    use super::super::command::LuaCommand;
    use super::super::stdio::StdioKind;
    use super::split_text;

    #[test]
    fn text_is_never_cut_inside_a_character() {
        let mut buf = "aé".as_bytes().to_vec();
        buf.truncate(2);
        assert_eq!(split_text(&mut buf, 100).as_deref(), Some("a"));
        assert_eq!(buf, "é".as_bytes()[..1]);
        assert_eq!(split_text(&mut buf, 100), None);
        buf.push("é".as_bytes()[1]);
        assert_eq!(split_text(&mut buf, 100).as_deref(), Some("é"));
        let mut buf = "héllo".as_bytes().to_vec();
        assert_eq!(split_text(&mut buf, 2).as_deref(), Some("h"));
        assert_eq!(split_text(&mut buf, 100).as_deref(), Some("éllo"));
        let mut buf = vec![0xFF, b'a'];
        assert_eq!(split_text(&mut buf, 100).as_deref(), Some("\u{FFFD}a"));
    }

    /// A command for `prog` with no command or file system restrictions.
    fn unrestricted(prog: String) -> LuaCommand {
        LuaCommand::new(
            prog,
            std::sync::Arc::new(crate::plugin::command_policy::CommandPolicy::Unrestricted),
            std::sync::Arc::new(crate::plugin::runtime::bindings::fs::FsPolicy::new(
                std::path::Path::new("demo"),
                true,
                false,
            )),
        )
    }

    fn echo() -> LuaCommand {
        let (prog, args) = if cfg!(windows) {
            ("cmd.exe".into(), vec!["/C".into(), "echo hello".into()])
        } else {
            ("echo".into(), vec!["hello".into()])
        };
        let mut cmd = unrestricted(prog);
        cmd.args = args;
        cmd.stdout = StdioKind::Piped;
        cmd.stderr = StdioKind::Piped;
        cmd
    }

    #[tokio::test]
    async fn spawn_then_wait_with_output_streams_stdout() {
        let child_cmd = echo();
        let mut child = child_cmd.spawn_inner().await.unwrap();
        assert!(child.id().is_some());
        let out = child.wait_with_output().await.unwrap();
        assert!(out.status.success);
        assert!(out.stdout.to_lowercase().contains("hello"));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn write_all_then_close_stdin_streams_through_cat() {
        let mut cmd = unrestricted("cat".into());
        cmd.stdin = StdioKind::Piped;
        cmd.stdout = StdioKind::Piped;
        let mut child = cmd.spawn_inner().await.unwrap();
        child.write_all(b"streamed\n").await.unwrap();
        child.close_stdin();
        let out = child.wait_with_output().await.unwrap();
        assert!(out.status.success);
        assert_eq!(out.stdout, "streamed\n");
    }
}
