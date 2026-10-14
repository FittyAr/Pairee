//! Spawn a shell command on a **real PTY** (see [`super::pty`]: Unix
//! `posix_openpt` / Windows ConPTY).
//!
//! This is not a VT emulator: bytes are split into lines and later rendered
//! with `ansi-to-tui`. The child sees `isatty(stdout) == true`.

use super::pty::{ChildWaiter, NativePty, PtyCommand, PtySize, PtySystem, SpawnedPty};
use super::pty_control::{Control, PtyControl, spawn_writer};
use anyhow::{Context, anyhow};
use std::io::Read;
use std::path::Path;
use std::time::{Duration, Instant};

/// Cursor position query ConPTY sends at start-up; stripped from output.
const DSR_QUERY: &str = "\u{1b}[6n";
/// Longest line kept whole: output without newlines (progress bars,
/// minified files) is split here instead of growing without bound.
const MAX_LINE_BYTES: usize = 64 * 1024;
/// After the child exits, output still arriving is read until the stream
/// ends or stays quiet this long (a background descendant may keep the
/// terminal open forever)...
const DRAIN_QUIET: Duration = Duration::from_millis(500);
/// ...and never for longer than this.
const DRAIN_MAX: Duration = Duration::from_secs(5);
/// A cancelled child still running this long after a kill is killed again.
const KILL_INSIST: Duration = Duration::from_secs(2);

/// Result of a PTY-backed shell command.
#[derive(Debug, Clone)]
pub struct PtyRunResult {
    pub output: String,
    pub success: bool,
}

/// Run `script` on a PTY and return the captured text (stdout+stderr mixed).
/// Once `cancelled` returns `true` the command is killed (see
/// [`PtyRun::wait_or_cancel`]) and ends unsuccessfully.
pub fn run_shell_on_pty(
    script: &str,
    cwd: Option<&Path>,
    cancelled: &dyn Fn() -> bool,
) -> anyhow::Result<PtyRunResult> {
    let mut lines = Vec::new();
    let (_control, run) = spawn_shell_on_pty(script, cwd, PtySize::DEFAULT)?;
    let success = run.wait_or_cancel(|line| lines.push(line), cancelled)?;
    Ok(PtyRunResult {
        output: lines.join("\n"),
        success,
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
        input: Some(spawn_writer(writer)),
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
    pub fn wait(self, on_line: impl FnMut(String)) -> anyhow::Result<bool> {
        self.wait_or_cancel(on_line, || false)
    }

    /// [`Self::wait`] that stops the child once `cancelled` turns `true`:
    /// it is killed, and killed again (harder, see [`PtyControl::kill`])
    /// every [`KILL_INSIST`] it keeps running.
    pub fn wait_or_cancel(
        self,
        mut on_line: impl FnMut(String),
        cancelled: impl Fn() -> bool,
    ) -> anyhow::Result<bool> {
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

        let mut killed_at: Option<Instant> = None;
        let status = loop {
            match done_rx.try_recv() {
                Ok(exit) => break exit.map_err(|e| anyhow!("wait pty child: {e}")),
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    break Err(anyhow!("pty wait thread ended"));
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {}
            }
            if killed_at.is_none_or(|at| at.elapsed() >= KILL_INSIST) && cancelled() {
                control.kill();
                killed_at = Some(Instant::now());
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
        drain_after_exit(&line_rx, &mut on_line, DRAIN_QUIET, DRAIN_MAX);
        status
    }
}

/// Passes on the lines still arriving after the child exited, until the
/// reader ends, `quiet` passes without a line, or `max` passes. A reader
/// still blocked (a descendant holding the terminal) is left behind.
fn drain_after_exit(
    line_rx: &std::sync::mpsc::Receiver<String>,
    on_line: &mut impl FnMut(String),
    quiet: Duration,
    max: Duration,
) {
    let deadline = Instant::now() + max;
    loop {
        let wait = quiet.min(deadline.saturating_duration_since(Instant::now()));
        match line_rx.recv_timeout(wait) {
            Ok(line) => on_line(line),
            Err(_) => return,
        }
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
                    let scanned = acc.len();
                    decode_utf8_chunk(&mut pending, &chunk[..n], &mut acc);
                    for line in take_complete_lines(&mut acc, scanned) {
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
        for line in take_complete_lines(&mut acc, 0) {
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

/// Strips ConPTY cursor queries and splits complete lines (`\n`, strip
/// `\r`) off `acc`; a line past [`MAX_LINE_BYTES`] is cut there. Only the
/// bytes from `scanned` on (those added since the last call) can hold a
/// newline, so each byte is searched once.
fn take_complete_lines(acc: &mut String, scanned: usize) -> Vec<String> {
    // A query split across chunks starts at most its length before.
    let mut from = floor_boundary(acc, scanned.saturating_sub(DSR_QUERY.len()));
    let mut search = scanned.min(acc.len());
    while let Some(i) = acc[from..].find(DSR_QUERY).map(|i| i + from) {
        acc.replace_range(i..i + DSR_QUERY.len(), "");
        // New bytes moved left into the removed query's place.
        search = search.min(i);
        from = i;
    }

    let mut lines = Vec::new();
    let mut start = 0;
    let mut search = floor_boundary(acc, search.min(acc.len()));
    while let Some(i) = acc[search..].find('\n').map(|i| i + search) {
        let mut line = &acc[start..i];
        line = line.strip_suffix('\r').unwrap_or(line);
        lines.push(line.to_string());
        start = i + 1;
        search = start;
    }
    acc.drain(..start);
    while acc.len() > MAX_LINE_BYTES {
        let cut = floor_boundary(acc, MAX_LINE_BYTES);
        lines.push(acc.drain(..cut).collect());
    }
    lines
}

/// The largest char boundary of `s` at or below `i`.
fn floor_boundary(s: &str, mut i: usize) -> usize {
    while !s.is_char_boundary(i) {
        i -= 1;
    }
    i
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn take_complete_lines_strips_cr_and_dsr() {
        let mut acc = String::from("hi\r\n\u{1b}[6nnext");
        let lines = take_complete_lines(&mut acc, 0);
        assert_eq!(lines, vec!["hi".to_string()]);
        assert_eq!(acc, "next");
    }

    #[test]
    fn take_complete_lines_finds_what_spans_chunks() {
        let mut acc = String::from("a\u{1b}[");
        assert!(take_complete_lines(&mut acc, 0).is_empty());
        let scanned = acc.len();
        acc.push_str("6nb\nc");
        assert_eq!(take_complete_lines(&mut acc, scanned), ["ab"]);
        assert_eq!(acc, "c");
    }

    #[test]
    fn a_line_without_newline_is_cut_instead_of_growing() {
        let mut acc = "é".repeat(MAX_LINE_BYTES); // two bytes each
        let lines = take_complete_lines(&mut acc, 0);
        assert!(lines.iter().all(|l| l.len() <= MAX_LINE_BYTES));
        assert!(acc.len() <= MAX_LINE_BYTES);
        assert_eq!(
            lines.iter().map(String::len).sum::<usize>() + acc.len(),
            2 * MAX_LINE_BYTES
        );
    }

    #[test]
    fn draining_stops_when_output_goes_quiet() {
        // A descendant keeps the terminal open: the sender never closes.
        let (tx, rx) = std::sync::mpsc::channel();
        tx.send("late".to_string()).unwrap();
        let started = Instant::now();
        let mut lines = Vec::new();
        drain_after_exit(
            &rx,
            &mut |l| lines.push(l),
            Duration::from_millis(50),
            Duration::from_secs(5),
        );
        assert_eq!(lines, ["late"]);
        assert!(started.elapsed() < Duration::from_secs(2));
        drop(tx);
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

    /// A child that runs until it is killed (then fails), counting kills.
    struct RunsUntilKilled(std::sync::Arc<std::sync::atomic::AtomicUsize>);

    struct CountingKiller(std::sync::Arc<std::sync::atomic::AtomicUsize>);

    impl crate::terminal::pty::ChildKiller for CountingKiller {
        fn kill(&mut self) -> std::io::Result<()> {
            self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        }
    }

    struct UntilKilled(std::sync::Arc<std::sync::atomic::AtomicUsize>);

    impl ChildWaiter for UntilKilled {
        fn wait(self: Box<Self>) -> std::io::Result<bool> {
            while self.0.load(std::sync::atomic::Ordering::SeqCst) == 0 {
                std::thread::sleep(Duration::from_millis(5));
            }
            Ok(false)
        }
    }

    impl PtySystem for RunsUntilKilled {
        fn spawn(&self, _: &PtyCommand) -> std::io::Result<SpawnedPty> {
            Ok(SpawnedPty {
                pid: Some(7),
                reader: Box::new(&b""[..]),
                writer: Box::new(std::io::sink()),
                master: Box::new(Inert),
                killer: Box::new(CountingKiller(self.0.clone())),
                waiter: Box::new(UntilKilled(self.0.clone())),
            })
        }
    }

    #[test]
    fn cancelling_kills_the_child_and_ends_the_wait() {
        let kills = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let fake = RunsUntilKilled(kills.clone());
        let (_control, run) = spawn_on(&fake, "long job", None, FAKE_SIZE).unwrap();
        let started = Instant::now();
        let cancel_after = Duration::from_millis(50);
        let ok = run
            .wait_or_cancel(|_| {}, || started.elapsed() >= cancel_after)
            .unwrap();
        assert!(!ok, "a killed command failed");
        assert_eq!(kills.load(std::sync::atomic::Ordering::SeqCst), 1);
        assert!(started.elapsed() < Duration::from_secs(2));
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
            let res = run_shell_on_pty("echo hello", None, &|| false);
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
            let res = run_shell_on_pty("test -t 1 && echo IS_TTY || echo NOT_TTY", None, &|| false);
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
