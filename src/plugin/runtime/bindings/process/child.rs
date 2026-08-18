//! Streaming `Child` userdata from `Command:spawn()`.

use super::output::{LuaOutput, LuaStatus};
use mlua::{UserData, UserDataMethods};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

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

    pub async fn read(&mut self, len: usize) -> mlua::Result<String> {
        let n = len.max(1);
        let mut buf = vec![0u8; n];
        let stdout = self
            .stdout
            .as_mut()
            .ok_or_else(|| mlua::Error::RuntimeError("child stdout is not piped".into()))?;
        let got = stdout
            .read(&mut buf)
            .await
            .map_err(|e| mlua::Error::RuntimeError(format!("read failed: {e}")))?;
        buf.truncate(got);
        Ok(String::from_utf8_lossy(&buf).into_owned())
    }

    pub async fn read_line(&mut self) -> mlua::Result<String> {
        let mut line = String::from_utf8_lossy(&self.leftover).into_owned();
        self.leftover.clear();
        if let Some(pos) = line.find('\n') {
            let rest = line.split_off(pos + 1);
            self.leftover = rest.into_bytes();
            return Ok(line);
        }
        let mut buf = [0u8; 256];
        while let Some(out) = self.stdout.as_mut() {
            let n = out
                .read(&mut buf)
                .await
                .map_err(|e| mlua::Error::RuntimeError(format!("read_line failed: {e}")))?;
            if n == 0 {
                break;
            }
            line.push_str(&String::from_utf8_lossy(&buf[..n]));
            if let Some(pos) = line.find('\n') {
                let rest = line.split_off(pos + 1);
                self.leftover = rest.into_bytes();
                break;
            }
        }
        Ok(line)
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
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        stdout.extend_from_slice(&self.leftover);
        self.leftover.clear();
        if let Some(mut out) = self.stdout.take() {
            out.read_to_end(&mut stdout)
                .await
                .map_err(|e| mlua::Error::RuntimeError(format!("read stdout failed: {e}")))?;
        }
        if let Some(mut err) = self.stderr.take() {
            err.read_to_end(&mut stderr)
                .await
                .map_err(|e| mlua::Error::RuntimeError(format!("read stderr failed: {e}")))?;
        }
        let status = self.wait().await?;
        Ok(LuaOutput {
            status,
            stdout: String::from_utf8_lossy(&stdout).into_owned(),
            stderr: String::from_utf8_lossy(&stderr).into_owned(),
        })
    }
}

impl UserData for LuaChild {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method("id", |_, this, ()| Ok(this.id()));
        methods.add_method_mut("start_kill", |_, this, ()| this.start_kill());
        methods.add_method_mut("try_wait", |_, this, ()| this.try_wait());
        methods.add_method_mut("close_stdin", |_, this, ()| {
            this.close_stdin();
            Ok(())
        });

        methods.add_async_method_mut("write_all", |_, this, src: String| async move {
            this.write_all(src.as_bytes()).await
        });
        methods.add_async_method_mut("read", |_, this, len: Option<usize>| async move {
            this.read(len.unwrap_or(4096)).await
        });
        methods.add_async_method_mut(
            "read_line",
            |_, this, ()| async move { this.read_line().await },
        );
        methods.add_async_method_mut("wait", |_, this, ()| async move { this.wait().await });
        methods.add_async_method_mut("wait_with_output", |_, this, ()| async move {
            this.wait_with_output().await
        });
    }
}

#[cfg(test)]
mod tests {
    use super::super::command::LuaCommand;
    use super::super::stdio::StdioKind;

    fn echo() -> LuaCommand {
        let (prog, args) = if cfg!(windows) {
            ("cmd.exe".into(), vec!["/C".into(), "echo hello".into()])
        } else {
            ("echo".into(), vec!["hello".into()])
        };
        let mut cmd = LuaCommand::new(prog, true, false);
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
        let mut cmd = LuaCommand::new("cat".into(), true, false);
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
