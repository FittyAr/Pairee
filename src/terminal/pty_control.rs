//! What the UI keeps of a running PTY child (see [`super::pty_cmd`]): its
//! pid, its input and a way to stop it.

use super::pty::{ChildKiller, PtyMaster, PtySize};
use crate::lock::LockExt;
use std::io::Write;
use std::sync::mpsc::{Sender, channel};
use std::sync::{Arc, Mutex, MutexGuard};

/// Shared between the UI (input, kill) and the thread waiting for the
/// child, which releases the PTY once the child exits.
///
/// Nothing that can block runs under the lock: input goes through a
/// channel to its own writer thread ([`spawn_writer`]), so a child that
/// does not read its input never stalls the UI (or a kill) behind a full
/// terminal buffer.
#[derive(Clone)]
pub struct PtyControl(Arc<Mutex<Control>>);

pub(super) struct Control {
    pub pid: Option<u32>,
    /// The writer thread's queue; `None` once released.
    pub input: Option<Sender<Vec<u8>>>,
    pub master: Option<Box<dyn PtyMaster>>,
    pub killer: Option<Box<dyn ChildKiller>>,
}

impl std::fmt::Debug for PtyControl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PtyControl")
            .field("pid", &self.pid())
            .finish()
    }
}

/// Writes what is queued on the returned sender into `writer`, on its own
/// thread, until the sender is dropped or a write fails. The writer (the
/// child's input) is closed when the thread ends: after [`PtyControl`]
/// released it, i.e. once the child exited (ConPTY needs it open until
/// then). A write blocked on a child that never reads is left behind
/// with its thread instead of blocking whoever typed.
pub(super) fn spawn_writer(mut writer: Box<dyn Write + Send>) -> Sender<Vec<u8>> {
    let (tx, rx) = channel::<Vec<u8>>();
    let spawned = std::thread::Builder::new()
        .name("pairee-pty-input".to_string())
        .spawn(move || {
            for bytes in rx {
                if writer
                    .write_all(&bytes)
                    .and_then(|()| writer.flush())
                    .is_err()
                {
                    break;
                }
            }
        });
    if let Err(e) = spawned {
        // The receiver went with the closure: every send now fails.
        log::warn!("pty input thread: {e}");
    }
    tx
}

impl PtyControl {
    pub(super) fn new(control: Control) -> Self {
        Self(Arc::new(Mutex::new(control)))
    }

    fn lock(&self) -> MutexGuard<'_, Control> {
        self.0.lock_safe()
    }

    pub fn pid(&self) -> Option<u32> {
        self.lock().pid
    }

    /// Queues `bytes` as input of the process (never blocks). `false` once
    /// it has exited or its input failed.
    pub fn send(&self, bytes: &[u8]) -> bool {
        self.lock()
            .input
            .as_ref()
            .is_some_and(|input| input.send(bytes.to_vec()).is_ok())
    }

    /// Tells the process its terminal is now `size` (no effect once it
    /// has exited).
    pub fn resize(&self, size: PtySize) {
        if let Some(master) = self.lock().master.as_ref()
            && let Err(e) = master.resize(size)
        {
            log::warn!("resize pty: {e}");
        }
    }

    /// Stops the process (no effect once it has exited). Asking again
    /// insists (on Unix the hang-up becomes `SIGKILL`).
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
    /// Closed outside the lock: closing a pseudo console may block until
    /// its last output is read, and the UI must not wait for that.
    pub(super) fn release(&self) {
        let released = {
            let mut control = self.lock();
            (
                control.input.take(),
                control.master.take(),
                control.killer.take(),
            )
        };
        drop(released);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc::Receiver;
    use std::time::Duration;

    /// A child input that blocks every write until `gate` lets it through.
    struct Stalled {
        gate: Receiver<()>,
        written: Sender<Vec<u8>>,
    }

    impl Write for Stalled {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            let _ = self.gate.recv();
            let _ = self.written.send(buf.to_vec());
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn a_child_that_does_not_read_never_blocks_input_or_kill() {
        let (open, gate) = channel();
        let (written, seen) = channel();
        let control = PtyControl::new(Control {
            pid: Some(1),
            input: Some(spawn_writer(Box::new(Stalled { gate, written }))),
            master: None,
            killer: None,
        });
        // The first write blocks in the writer thread; these return at once.
        assert!(control.send(b"a"));
        assert!(control.send(b"b"));
        control.kill();
        assert_eq!(control.pid(), Some(1), "the lock is free");

        open.send(()).unwrap();
        open.send(()).unwrap();
        let got: Vec<_> = (0..2)
            .map(|_| seen.recv_timeout(Duration::from_secs(5)).unwrap())
            .collect();
        assert_eq!(got, [b"a".to_vec(), b"b".to_vec()]);

        control.release();
        assert!(!control.send(b"c"), "no input once released");
    }
}
