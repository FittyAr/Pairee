//! Spawn a shell command on a **real PTY** (see [`super::pty`]: Unix
//! `posix_openpt` / Windows ConPTY).
//!
//! This is not a VT emulator: bytes are split into lines and later rendered
//! with `ansi-to-tui`. The child sees `isatty(stdout) == true`.

use super::pty::{ChildWaiter, NativePty, PtyCommand, PtySize, PtySystem, SpawnedPty};
use super::pty_control::{Control, PtyControl};
use anyhow::{Context, anyhow};
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
    on_line: impl FnMut(String),
) -> anyhow::Result<PtyRunResult> {
    let (control, run) = spawn_shell_on_pty(script, cwd, PtySize::DEFAULT)?;
    let success = run.wait(on_line)?;
    Ok(PtyRunResult {
        output: String::new(),
        success,
        pid: control.pid(),
    })
}

/// Starts `script` on a new PTY of `size`. The [`PtyControl`] types into,
/// resizes and stops the process; [`PtyRun::wait`] streams its output until
/// it exits.
pub fn spawn_shell_on_pty(
    script: &str,
    cwd: Option<&Path>,
    size: PtySize,
) -> anyhow::Result<(PtyControl, PtyRun)> {
    spawn_on(&NativePty, script, cwd, size)
}

fn spawn_on(
    system: &dyn PtySystem,
    script: &str,
    cwd: Option<&Path>,
    size: PtySize,
) -> anyhow::Result<(PtyControl, PtyRun)> {
    if script.trim().is_empty() {
        anyhow::bail!("empty command");
    }
    let SpawnedPty {
        pid,
        reader,
        writer,
        master,
        killer,
        waiter,
    } = system
        .spawn(&shell_command(script, cwd, size))
        .with_context(|| format!("spawn in pty: {script}"))?;
    let control = PtyControl::new(Control {
        pid,
        writer: Some(writer),
        master: Some(master),
        killer: Some(killer),
    });
    let run = PtyRun {
        waiter,
        reader,
        control: control.clone(),
    };
    Ok((control, run))
}

/// A spawned PTY child whose output is still to be read.
pub struct PtyRun {
    waiter: Box<dyn ChildWaiter>,
    reader: Box<dyn Read + Send>,
    control: PtyControl,
}

impl PtyRun {
    /// Calls `on_line` with every complete output line until the child
    /// exits; returns whether it succeeded.
    pub fn wait(self, mut on_line: impl FnMut(String)) -> anyhow::Result<bool> {
        let PtyRun {
            waiter,
            reader,
            control,
        } = self;
        let line_rx = read_lines(reader);
        let (done_tx, done_rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let _ = done_tx.send(waiter.wait());
        });

        let status = loop {
            match done_rx.try_recv() {
                Ok(exit) => break exit.map_err(|e| anyhow!("wait pty child: {e}")),
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    break Err(anyhow!("pty wait thread ended"));
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
        control.release();
        while let Ok(line) = line_rx.recv() {
            on_line(line);
        }
        status
    }
}

/// Reads `reader` on a side thread (so PTY buffers cannot deadlock against
/// `wait()`) and sends every complete line.
fn read_lines(mut reader: Box<dyn Read + Send>) -> std::sync::mpsc::Receiver<String> {
    let (line_tx, line_rx) = std::sync::mpsc::channel();
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
    line_rx
}

/// The PTY layer quotes every argument itself, so the shell invocation
/// comes from `crate::shell::shell_invocation` (script passed via the
/// environment on Windows instead of an argument cmd.exe would mis-parse).
fn shell_command(script: &str, cwd: Option<&Path>, size: PtySize) -> PtyCommand {
    let inv = crate::shell::shell_invocation(script);
    let mut env = inv.env;
    env.push(("TERM".to_string(), "xterm-256color".to_string()));
    env.push(("COLORTERM".to_string(), "truecolor".to_string()));
    PtyCommand {
        program: inv.program,
        args: inv.args,
        env,
        cwd: cwd.map(Path::to_path_buf),
        size,
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

    /// A child that printed `output` and exited with `success`.
    struct FakePty {
        output: &'static [u8],
        success: bool,
    }

    struct Inert;

    impl crate::terminal::pty::PtyMaster for Inert {
        fn resize(&self, _: PtySize) -> std::io::Result<()> {
            Ok(())
        }
    }

    impl crate::terminal::pty::ChildKiller for Inert {
        fn kill(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    struct Exit(bool);

    impl ChildWaiter for Exit {
        fn wait(self: Box<Self>) -> std::io::Result<bool> {
            Ok(self.0)
        }
    }

    const FAKE_SIZE: PtySize = PtySize { rows: 9, cols: 33 };

    impl PtySystem for FakePty {
        fn spawn(&self, cmd: &PtyCommand) -> std::io::Result<SpawnedPty> {
            assert!(
                cmd.env
                    .iter()
                    .any(|(k, v)| k == "TERM" && v == "xterm-256color")
            );
            assert_eq!(cmd.size, FAKE_SIZE);
            Ok(SpawnedPty {
                pid: Some(42),
                reader: Box::new(self.output),
                writer: Box::new(std::io::sink()),
                master: Box::new(Inert),
                killer: Box::new(Inert),
                waiter: Box::new(Exit(self.success)),
            })
        }
    }

    fn fake_run(output: &'static [u8], success: bool) -> (Vec<String>, bool, PtyControl) {
        let fake = FakePty { output, success };
        let (control, run) = spawn_on(&fake, "anything", None, FAKE_SIZE).unwrap();
        let mut lines = Vec::new();
        let ok = run.wait(|line| lines.push(line)).unwrap();
        (lines, ok, control)
    }

    #[test]
    fn fake_pty_streams_lines_and_exit_status() {
        let (lines, ok, control) = fake_run(b"one\r\ntwo\n\xc3", true);
        assert_eq!(lines, ["one", "two", "\u{FFFD}"]);
        assert!(ok);
        assert_eq!(control.pid(), Some(42));
        // Released once the child exited: no more input, resize or kill.
        assert!(!control.send(b"x"));
        control.resize(PtySize::DEFAULT);
        control.kill();
    }

    #[test]
    fn fake_pty_reports_failure() {
        let (lines, ok, _) = fake_run(b"", false);
        assert!(lines.is_empty());
        assert!(!ok);
    }

    #[test]
    fn empty_script_is_refused() {
        let fake = FakePty {
            output: b"",
            success: true,
        };
        assert!(spawn_on(&fake, "  ", None, FAKE_SIZE).is_err());
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
