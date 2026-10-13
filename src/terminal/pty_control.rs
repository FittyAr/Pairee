//! What the UI keeps of a running PTY child (see [`super::pty_cmd`]): its
//! pid, its input and a way to stop it.

use portable_pty::{ChildKiller, MasterPty};
use std::io::Write;
use std::sync::{Arc, Mutex, MutexGuard};

/// Shared between the UI (input, kill) and the thread waiting for the
/// child, which releases the PTY once the child exits.
#[derive(Clone)]
pub struct PtyControl(Arc<Mutex<Control>>);

pub(super) struct Control {
    pub pid: Option<u32>,
    pub writer: Option<Box<dyn Write + Send>>,
    pub master: Option<Box<dyn MasterPty + Send>>,
    pub killer: Option<Box<dyn ChildKiller + Send + Sync>>,
}

impl std::fmt::Debug for PtyControl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PtyControl")
            .field("pid", &self.pid())
            .finish()
    }
}

impl PtyControl {
    pub(super) fn new(control: Control) -> Self {
        Self(Arc::new(Mutex::new(control)))
    }

    fn lock(&self) -> MutexGuard<'_, Control> {
        self.0.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn pid(&self) -> Option<u32> {
        self.lock().pid
    }

    /// Types `bytes` into the process. `false` once it has exited.
    pub fn send(&self, bytes: &[u8]) -> bool {
        let mut control = self.lock();
        let Some(writer) = control.writer.as_mut() else {
            return false;
        };
        writer
            .write_all(bytes)
            .and_then(|()| writer.flush())
            .is_ok()
    }

    /// Stops the process (no effect once it has exited).
    pub fn kill(&self) {
        if let Some(killer) = self.lock().killer.as_mut()
            && let Err(e) = killer.kill()
        {
            log::warn!("kill pty child: {e}");
        }
    }

    /// Closes the input and the PTY so the output reader sees EOF. Done
    /// once the child exited: ConPTY must keep stdin open until then, or
    /// cmd.exe may exit before `/C` runs.
    pub(super) fn release(&self) {
        let mut control = self.lock();
        control.writer = None;
        control.master = None;
        control.killer = None;
    }
}
