//! Unix PTY: `posix_openpt` master, the child gets the slave as its
//! controlling terminal (new session + `TIOCSCTTY`), size via `TIOCSWINSZ`.

use super::{ChildKiller, ChildWaiter, PtyCommand, PtyMaster, PtySize, SpawnedPty};
use crate::lock::LockExt;
use rustix::fs::{Mode, OFlags};
use rustix::io::FdFlags;
use rustix::process::{Pid, Signal};
use rustix::pty::OpenptFlags;
use rustix::termios::Winsize;
use std::fs::File;
use std::io;
use std::os::fd::{AsFd, OwnedFd};
use std::os::unix::process::CommandExt;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub(super) fn spawn(cmd: &PtyCommand) -> io::Result<SpawnedPty> {
    let master = open_master()?;
    // macOS refuses terminal ioctls on a master whose slave was never
    // opened (ENOTTY), so the size is set once the slave is open.
    let slave = open_slave(&master)?;
    set_size(&slave, cmd.size)?;
    let child = command(cmd, slave)?.spawn()?;
    let pid = child.id();
    let master = File::from(master);
    let child = Arc::new(Mutex::new(ChildSlot {
        child,
        reaped: false,
    }));
    Ok(SpawnedPty {
        pid: Some(pid),
        reader: Box::new(master.try_clone()?),
        writer: Box::new(master.try_clone()?),
        master: Box::new(Master(master)),
        killer: Box::new(Killer {
            child: Arc::clone(&child),
            hung_up: false,
        }),
        waiter: Box::new(Waiter(child)),
    })
}

fn open_master() -> io::Result<OwnedFd> {
    let master = rustix::pty::openpt(OpenptFlags::RDWR | OpenptFlags::NOCTTY)?;
    // `posix_openpt` has no portable CLOEXEC flag; no child may inherit it.
    rustix::io::fcntl_setfd(&master, FdFlags::CLOEXEC)?;
    rustix::pty::grantpt(&master)?;
    rustix::pty::unlockpt(&master)?;
    Ok(master)
}

fn open_slave(master: &OwnedFd) -> io::Result<OwnedFd> {
    let path = rustix::pty::ptsname(master, Vec::new())?;
    let flags = OFlags::RDWR | OFlags::NOCTTY | OFlags::CLOEXEC;
    Ok(rustix::fs::open(path.as_c_str(), flags, Mode::empty())?)
}

fn set_size(master: impl AsFd, size: PtySize) -> io::Result<()> {
    let winsize = Winsize {
        ws_row: size.rows,
        ws_col: size.cols,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    Ok(rustix::termios::tcsetwinsize(master, winsize)?)
}

/// `cmd` with the slave as stdin, stdout and stderr and as the controlling
/// terminal of a new session (so job control and `SIGWINCH` work).
fn command(cmd: &PtyCommand, slave: OwnedFd) -> io::Result<Command> {
    let mut command = Command::new(&cmd.program);
    command
        .args(&cmd.args)
        .envs(cmd.env.iter().map(|(k, v)| (k, v)))
        .stdin(Stdio::from(slave.try_clone()?))
        .stdout(Stdio::from(slave.try_clone()?))
        .stderr(Stdio::from(slave.try_clone()?));
    if let Some(dir) = &cmd.cwd {
        command.current_dir(dir);
    }
    // SAFETY: the hook runs in the forked child before `exec`; it only makes
    // two raw syscalls (`setsid`, `ioctl`), which are async-signal-safe, and
    // neither allocates nor takes a lock. `slave` is CLOEXEC, so the copy the
    // closure owns is closed by `exec`.
    unsafe {
        command.pre_exec(move || {
            rustix::process::setsid()?;
            rustix::process::ioctl_tiocsctty(&slave)?;
            Ok(())
        });
    }
    Ok(command)
}

struct Master(File);

impl PtyMaster for Master {
    fn resize(&self, size: PtySize) -> io::Result<()> {
        set_size(&self.0, size)
    }
}

/// How often [`Waiter`] checks whether the child exited.
const WAIT_POLL: Duration = Duration::from_millis(20);

/// The child, shared by the [`Waiter`] (the only one that reaps it) and
/// the [`Killer`]. Reaping happens under the lock and the killer checks
/// `reaped` under it too, so a signal never reaches a recycled PID: until
/// it is reaped, the child (even a zombie) keeps its PID and its process
/// group ID.
struct ChildSlot {
    child: Child,
    reaped: bool,
}

type SharedChild = Arc<Mutex<ChildSlot>>;

/// Hangs up the child's whole process group (it leads a session of its
/// own, so the group is the shell and every job it started), like closing
/// its terminal window would. They exit unless they handle `SIGHUP`;
/// asking again sends `SIGKILL`.
struct Killer {
    child: SharedChild,
    hung_up: bool,
}

impl ChildKiller for Killer {
    fn kill(&mut self) -> io::Result<()> {
        let slot = self.child.lock_safe();
        if slot.reaped {
            return Ok(());
        }
        let pid = i32::try_from(slot.child.id())
            .ok()
            .and_then(Pid::from_raw)
            .ok_or_else(|| io::Error::from(io::ErrorKind::InvalidInput))?;
        let signal = if self.hung_up {
            Signal::KILL
        } else {
            Signal::HUP
        };
        self.hung_up = true;
        // The group is the child's PID (`setsid` in `command`). The child
        // is signalled directly too: should the group be gone, or the
        // platform refuse the group signal (seen on macOS runners), the
        // child itself still gets it.
        let group = rustix::process::kill_process_group(pid, signal);
        let child = rustix::process::kill_process(pid, signal);
        group.or(child)?;
        Ok(())
    }
}

struct Waiter(SharedChild);

impl ChildWaiter for Waiter {
    /// Polls instead of blocking in `wait()`: a blocking wait would reap
    /// the child without the lock, racing the killer.
    fn wait(self: Box<Self>) -> io::Result<bool> {
        loop {
            {
                let mut slot = self.0.lock_safe();
                if let Some(status) = slot.child.try_wait()? {
                    slot.reaped = true;
                    return Ok(status.success());
                }
            }
            std::thread::sleep(WAIT_POLL);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    fn run(script: &str, size: PtySize) -> String {
        let cmd = PtyCommand {
            program: "sh".into(),
            args: vec!["-c".into(), script.into()],
            env: vec![("PAIREE_PTY_TEST".into(), "on".into())],
            cwd: None,
            size,
        };
        let mut pty = spawn(&cmd).expect("spawn on pty");
        let mut out = Vec::new();
        // EIO once the child exited and closed the slave: the end of output.
        let _ = pty.reader.read_to_end(&mut out);
        assert!(pty.waiter.wait().expect("wait"));
        String::from_utf8_lossy(&out).into_owned()
    }

    #[test]
    fn child_sees_the_requested_size_and_env() {
        let out = run(
            "stty size; echo $PAIREE_PTY_TEST",
            PtySize {
                rows: 33,
                cols: 101,
            },
        );
        assert!(out.contains("33 101"), "{out:?}");
        assert!(out.contains("on"), "{out:?}");
    }

    #[test]
    fn child_has_a_controlling_terminal() {
        let out = run("tty >/dev/null && echo CTTY_OK", PtySize::DEFAULT);
        assert!(out.contains("CTTY_OK"), "{out:?}");
    }

    #[test]
    fn kill_hangs_up_the_group_then_insists_and_spares_a_reaped_pid() {
        let cmd = PtyCommand {
            program: "sh".into(),
            // Ignores SIGHUP, and so does `sleep`, which inherits that.
            args: vec!["-c".into(), "trap '' HUP; echo READY; sleep 30".into()],
            env: Vec::new(),
            cwd: None,
            size: PtySize::DEFAULT,
        };
        let SpawnedPty {
            mut killer,
            waiter,
            mut reader,
            ..
        } = spawn(&cmd).expect("spawn on pty");
        // Signal only once the trap is set (a fixed sleep raced it on a
        // loaded machine, so SIGHUP killed the shell).
        let mut seen = Vec::new();
        let mut byte = [0u8; 1];
        while !String::from_utf8_lossy(&seen).contains("READY") {
            assert_eq!(reader.read(&mut byte).expect("read"), 1, "{seen:?}");
            seen.push(byte[0]);
        }
        // Keep reading, as Pairee does: on macOS a dying process can wait
        // for its terminal's unread output to drain before it exits.
        std::thread::spawn(move || {
            let _ = std::io::copy(&mut reader, &mut std::io::sink());
        });
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(waiter.wait());
        });
        killer.kill().expect("SIGHUP to the group");
        assert!(
            rx.recv_timeout(Duration::from_millis(300)).is_err(),
            "SIGHUP is ignored"
        );
        killer.kill().expect("SIGKILL to the group");
        let exit = rx.recv_timeout(Duration::from_secs(5)).expect("killed");
        assert!(!exit.expect("wait"));
        // Reaped: its PID may belong to someone else now, so no signal.
        killer.kill().expect("no-op once reaped");
    }

    #[test]
    fn resize_reaches_the_terminal() {
        let master = open_master().unwrap();
        let slave = open_slave(&master).unwrap();
        let pty = Master(File::from(master));
        let size = PtySize {
            rows: 40,
            cols: 120,
        };
        pty.resize(size).unwrap();
        let got = rustix::termios::tcgetwinsize(&slave).unwrap();
        assert_eq!((got.ws_row, got.ws_col), (40, 120));
    }
}
