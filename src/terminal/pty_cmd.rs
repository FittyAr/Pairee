//! Spawn a shell command on a **real PTY** (Unix `openpty` / Windows ConPTY).
//!
//! This is not a VT emulator: bytes are split into lines and later rendered
//! with `ansi-to-tui`. The child sees `isatty(stdout) == true`.

use anyhow::{Context, anyhow};
use portable_pty::{CommandBuilder, PtySize, native_pty_system};
use std::io::{Read, Write};
use std::path::Path;

const DSR_QUERY: &str = "\u{1b}[6n";
const DSR_REPLY: &[u8] = b"\x1b[1;1R";

/// Result of a PTY-backed shell command.
#[derive(Debug, Clone)]
pub struct PtyRunResult {
    pub output: String,
    pub success: bool,
    pub pid: Option<u32>,
}

/// Run `script` on a PTY and return the captured text (stdout+stderr mixed).
pub fn run_shell_on_pty(script: &str, cwd: Option<&Path>) -> anyhow::Result<PtyRunResult> {
    let mut lines = Vec::new();
    let result = stream_shell_on_pty(script, cwd, |line| lines.push(line))?;
    Ok(PtyRunResult {
        output: lines.join("\n"),
        success: result.success,
        pid: result.pid,
    })
}

/// Stream complete lines from a PTY-backed shell command.
pub fn stream_shell_on_pty(
    script: &str,
    cwd: Option<&Path>,
    mut on_line: impl FnMut(String),
) -> anyhow::Result<PtyRunResult> {
    if script.trim().is_empty() {
        anyhow::bail!("empty command");
    }

    let pty_system = native_pty_system();
    let pair = pty_system
        .openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })
        .context("open pty")?;

    let mut cmd = shell_command(script);
    cmd.env("TERM", "xterm-256color");
    cmd.env("COLORTERM", "truecolor");
    if let Some(dir) = cwd {
        cmd.cwd(dir);
    }

    let mut child = pair
        .slave
        .spawn_command(cmd)
        .with_context(|| format!("spawn in pty: {script}"))?;
    let pid = child.process_id();
    drop(pair.slave);

    // Read on a side thread so PTY buffers cannot deadlock against wait().
    let (line_tx, line_rx) = std::sync::mpsc::channel();
    let mut reader = pair.master.try_clone_reader().context("pty reader")?;
    std::thread::spawn(move || {
        let mut pending = Vec::new();
        let mut acc = String::new();
        let mut chunk = [0u8; 4096];
        loop {
            match reader.read(&mut chunk) {
                Ok(0) => break,
                Ok(n) => {
                    decode_utf8_chunk(&mut pending, &chunk[..n], &mut acc);
                    for line in take_complete_lines(&mut acc, None) {
                        if line_tx.send(line).is_err() {
                            return;
                        }
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => break,
            }
        }
        if !pending.is_empty() {
            acc.push_str(&String::from_utf8_lossy(&pending));
        }
        for line in take_complete_lines(&mut acc, None) {
            let _ = line_tx.send(line);
        }
        if !acc.is_empty() {
            let _ = line_tx.send(acc);
        }
    });

    // Take the writer so we can EOF the child after it exits. Do not drop it
    // before wait(): ConPTY would close stdin and cmd.exe can exit before
    // `/C` runs. Do not read on this thread while holding it (deadlock).
    let writer = pair.master.take_writer().context("pty writer")?;
    let (done_tx, done_rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = done_tx.send(child.wait());
    });

    let status = loop {
        match done_rx.try_recv() {
            Ok(st) => break st.map_err(|e| anyhow!("wait pty child: {e}"))?,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                anyhow::bail!("pty wait thread ended");
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {}
        }
        match line_rx.recv_timeout(std::time::Duration::from_millis(16)) {
            Ok(line) => on_line(line),
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                std::thread::sleep(std::time::Duration::from_millis(16));
            }
        }
    };
    drop(writer);
    drop(pair.master);

    while let Ok(line) = line_rx.recv() {
        on_line(line);
    }

    Ok(PtyRunResult {
        output: String::new(),
        success: status.success(),
        pid,
    })
}

fn shell_command(script: &str) -> CommandBuilder {
    #[cfg(windows)]
    {
        let mut cmd = CommandBuilder::new("cmd.exe");
        cmd.arg("/C");
        cmd.arg(script);
        cmd
    }
    #[cfg(not(windows))]
    {
        let mut cmd = CommandBuilder::new("sh");
        cmd.arg("-c");
        cmd.arg(script);
        cmd
    }
}

fn decode_utf8_chunk(pending: &mut Vec<u8>, chunk: &[u8], acc: &mut String) {
    pending.extend_from_slice(chunk);
    loop {
        match std::str::from_utf8(pending) {
            Ok(s) => {
                acc.push_str(s);
                pending.clear();
                return;
            }
            Err(err) => {
                let valid = err.valid_up_to();
                if valid > 0 {
                    acc.push_str(&String::from_utf8_lossy(&pending[..valid]));
                    pending.drain(..valid);
                    continue;
                }
                if err.error_len().is_some() {
                    acc.push('\u{FFFD}');
                    pending.remove(0);
                    continue;
                }
                return;
            }
        }
    }
}

/// Reply to ConPTY cursor queries and split complete lines (`\n`, strip `\r`).
pub(crate) fn take_complete_lines(
    acc: &mut String,
    mut reply: Option<&mut dyn Write>,
) -> Vec<String> {
    while let Some(i) = acc.find(DSR_QUERY) {
        if let Some(w) = reply.as_mut() {
            let _ = w.write_all(DSR_REPLY);
            let _ = w.flush();
        }
        acc.replace_range(i..i + DSR_QUERY.len(), "");
    }

    let mut lines = Vec::new();
    while let Some(i) = acc.find('\n') {
        let mut line: String = acc.drain(..=i).collect();
        line.pop();
        if line.ends_with('\r') {
            line.pop();
        }
        lines.push(line);
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn take_complete_lines_strips_cr_and_dsr() {
        let mut acc = String::from("hi\r\n\u{1b}[6nnext");
        let lines = take_complete_lines(&mut acc, None);
        assert_eq!(lines, vec!["hi".to_string()]);
        assert_eq!(acc, "next");
    }

    #[test]
    fn take_complete_lines_replies_to_dsr() {
        let mut acc = String::from("\u{1b}[6nline\n");
        let mut replied = Vec::new();
        let lines = take_complete_lines(&mut acc, Some(&mut replied));
        assert_eq!(lines, vec!["line".to_string()]);
        assert_eq!(replied, DSR_REPLY);
        assert!(acc.is_empty());
    }

    fn run_echo_hello() -> PtyRunResult {
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let res = run_shell_on_pty("echo hello", None);
            let _ = tx.send(res);
        });
        match rx.recv_timeout(Duration::from_secs(15)) {
            Ok(Ok(r)) => r,
            Ok(Err(e)) => panic!("pty echo failed: {e:#}"),
            Err(_) => panic!("pty echo timed out (possible ConPTY hang)"),
        }
    }

    #[test]
    fn pty_echo_hello_captures_output() {
        let result = run_echo_hello();
        assert!(
            result.output.to_lowercase().contains("hello"),
            "expected hello from PTY echo, got {:?}",
            result.output
        );
        assert!(result.success, "echo should exit 0");
    }

    #[cfg(unix)]
    #[test]
    fn pty_stdout_is_a_tty() {
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let res = run_shell_on_pty("test -t 1 && echo IS_TTY || echo NOT_TTY", None);
            let _ = tx.send(res);
        });
        let result = match rx.recv_timeout(Duration::from_secs(15)) {
            Ok(Ok(r)) => r,
            Ok(Err(e)) => panic!("pty isatty failed: {e:#}"),
            Err(_) => panic!("pty isatty timed out"),
        };
        assert!(
            result.output.contains("IS_TTY"),
            "child stdout should be a TTY, got {:?}",
            result.output
        );
    }
}
