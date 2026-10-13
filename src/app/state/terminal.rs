//! Terminal screens: the output of a `command &` run on a PTY (or of a
//! Transfer Engine apply command), with a bounded scrollback, the process
//! handle to type into or stop it, and the job that waits for it.

use crate::app::jobs::JobSlot;
use crate::terminal::pty::PtySize;
use crate::terminal::pty_control::PtyControl;
use std::collections::VecDeque;
use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard};

/// Output lines kept when no limit is configured.
pub const DEFAULT_LINES: usize = 10_000;

/// Output lines, oldest first, at most `limit` of them.
#[derive(Debug, Default)]
struct Lines {
    lines: VecDeque<String>,
    limit: usize,
    /// Bumped by every change, so the UI knows when to repaint.
    version: u64,
}

/// Output of a terminal screen, filled by the thread reading the process.
#[derive(Debug, Clone)]
pub struct TermOutput(Arc<Mutex<Lines>>);

impl TermOutput {
    pub fn new(limit: usize) -> Self {
        Self(Arc::new(Mutex::new(Lines {
            limit: limit.max(1),
            ..Lines::default()
        })))
    }

    fn lock(&self) -> MutexGuard<'_, Lines> {
        self.0.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Appends `line`, dropping the oldest one past the limit.
    pub fn push(&self, line: String) {
        let mut out = self.lock();
        if out.lines.len() >= out.limit {
            out.lines.pop_front();
        }
        out.lines.push_back(line);
        out.version += 1;
    }

    pub fn len(&self) -> usize {
        self.lock().lines.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn version(&self) -> u64 {
        self.lock().version
    }

    /// Every kept line (to open the output in the viewer).
    pub fn all(&self) -> Vec<String> {
        self.lock().lines.iter().cloned().collect()
    }

    /// The `height` lines ending `scroll` lines above the last one.
    pub fn window(&self, height: usize, scroll: usize) -> Vec<String> {
        let out = self.lock();
        let end = out.lines.len().saturating_sub(scroll);
        let start = end.saturating_sub(height);
        out.lines.range(start..end).cloned().collect()
    }
}

/// Where a terminal screen's process is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TermStatus {
    Running,
    /// Exited (`true` when it succeeded).
    Exited(bool),
}

#[derive(Debug)]
pub struct TerminalState {
    pub command: String,
    pub output: TermOutput,
    pub status: TermStatus,
    /// Lines scrolled back from the end of the output (0 = following it).
    pub scroll: usize,
    /// When set, Transfer Engine apply-command output is routed here.
    pub job_id: Option<uuid::Uuid>,
    process: Option<PtyControl>,
    /// Waits for the process; its result is the exit status.
    run: JobSlot<Result<bool, String>>,
    seen_version: u64,
}

impl TerminalState {
    fn new(command: String, limit: usize) -> Self {
        Self {
            command,
            output: TermOutput::new(limit),
            status: TermStatus::Running,
            scroll: 0,
            job_id: None,
            process: None,
            run: JobSlot::default(),
            seen_version: 0,
        }
    }

    /// A screen fed by the Transfer Engine job `job_id`.
    pub fn for_job(command: String, job_id: uuid::Uuid, limit: usize) -> Self {
        let mut ts = Self::new(command, limit);
        ts.job_id = Some(job_id);
        ts
    }

    /// Starts `command` on a PTY in `cwd`; a spawn failure is shown as the
    /// screen's output.
    pub fn spawn(command: String, cwd: &Path, limit: usize) -> Self {
        let mut ts = Self::new(command, limit);
        let size = PtySize::of_terminal();
        match crate::terminal::pty_cmd::spawn_shell_on_pty(&ts.command, Some(cwd), size) {
            Ok((control, run)) => {
                ts.process = Some(control);
                let output = ts.output.clone();
                ts.run.start(move |_| {
                    run.wait(|line| {
                        output.push(line);
                        crate::app::jobs::wake_event_loop();
                    })
                    .map_err(|e| e.to_string())
                });
            }
            Err(e) => {
                let text = crate::config::localization::t("terminal_spawn_failed");
                ts.output.push(text.replacen("{}", &format!("{e:#}"), 1));
                ts.status = TermStatus::Exited(false);
            }
        }
        ts
    }

    pub fn is_running(&self) -> bool {
        self.status == TermStatus::Running
    }

    /// Process id of the running command (none for Transfer Engine jobs).
    pub fn pid(&self) -> Option<u32> {
        self.process.as_ref().and_then(PtyControl::pid)
    }

    /// Stops the process, if any is still running.
    pub fn kill(&self) {
        if self.is_running()
            && let Some(process) = &self.process
        {
            process.kill();
        }
    }

    /// Gives the running process the new size of Pairee's terminal.
    pub fn resize(&self, size: PtySize) {
        if self.is_running()
            && let Some(process) = &self.process
        {
            process.resize(size);
        }
    }

    /// Types `bytes` into the running process. `false` when it cannot take
    /// input (exited, or a Transfer Engine job).
    pub fn send_input(&self, bytes: &[u8]) -> bool {
        self.is_running() && self.process.as_ref().is_some_and(|p| p.send(bytes))
    }

    /// Ends a Transfer Engine job screen.
    pub fn job_finished(&mut self) {
        self.status = TermStatus::Exited(true);
    }

    /// Takes in what the process did since the last call; `true` when the
    /// screen must be repainted.
    pub fn poll(&mut self) -> bool {
        let mut changed = false;
        if let Some(result) = self.run.poll() {
            if let Err(e) = &result {
                self.output.push(e.clone());
            }
            self.status = TermStatus::Exited(result.unwrap_or(false));
            changed = true;
        }
        let version = self.output.version();
        if version != self.seen_version {
            self.seen_version = version;
            changed = true;
        }
        changed
    }

    /// Scrolls back (`up`) or forward by `lines`, within the output.
    pub fn scroll_by(&mut self, lines: usize, up: bool) {
        self.scroll = if up {
            (self.scroll + lines).min(self.output.len().saturating_sub(1))
        } else {
            self.scroll.saturating_sub(lines)
        };
    }
}

/// A screen that goes away (closed, or the app quits) stops its process.
impl Drop for TerminalState {
    fn drop(&mut self) {
        self.kill();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_keeps_the_newest_lines_up_to_the_limit() {
        let out = TermOutput::new(3);
        for i in 0..5 {
            out.push(i.to_string());
        }
        assert_eq!(out.all(), vec!["2", "3", "4"]);
        assert_eq!(out.window(2, 0), vec!["3", "4"]);
        assert_eq!(out.window(2, 1), vec!["2", "3"]);
        assert_eq!(out.window(5, 9), Vec::<String>::new());
    }

    #[test]
    fn scrolling_stays_inside_the_output() {
        let mut ts = TerminalState::for_job("x".into(), uuid::Uuid::new_v4(), 100);
        for i in 0..10 {
            ts.output.push(i.to_string());
        }
        assert!(ts.poll(), "new output repaints");
        assert!(!ts.poll());
        ts.scroll_by(50, true);
        assert_eq!(ts.scroll, 9);
        ts.scroll_by(4, false);
        assert_eq!(ts.scroll, 5);
        assert!(!ts.send_input(b"x"), "job screens take no input");
        ts.job_finished();
        assert!(!ts.is_running());
    }
}
