//! Unix PTY: `posix_openpt` master, the child gets the slave as its
//! controlling terminal (new session + `TIOCSCTTY`), size via `TIOCSWINSZ`.

use super::{ChildKiller, ChildWaiter, PtyCommand, PtyMaster, PtySize, SpawnedPty};
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

pub(super) fn spawn(cmd: &PtyCommand) -> io::Result<SpawnedPty> {
    let master = open_master()?;
    set_size(&master, cmd.size)?;
    let slave = open_slave(&master)?;
    let child = command(cmd, slave)?.spawn()?;
    let pid = child.id();
    let master = File::from(master);
    Ok(SpawnedPty {
        pid: Some(pid),
        reader: Box::new(master.try_clone()?),
        writer: Box::new(master.try_clone()?),
        master: Box::new(Master(master)),
        killer: Box::new(Killer(pid)),
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

/// Hangs the child up, like closing its terminal window would: the shell
/// and its foreground job exit unless they handle `SIGHUP`.
struct Killer(u32);

impl ChildKiller for Killer {
    fn kill(&mut self) -> io::Result<()> {
        let pid = i32::try_from(self.0)
            .ok()
            .and_then(Pid::from_raw)
            .ok_or_else(|| io::Error::from(io::ErrorKind::InvalidInput))?;
        Ok(rustix::process::kill_process(pid, Signal::HUP)?)
    }
}

struct Waiter(Child);

impl ChildWaiter for Waiter {
    fn wait(mut self: Box<Self>) -> io::Result<bool> {
        Ok(self.0.wait()?.success())
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
    fn resize_reaches_the_terminal() {
        let master = open_master().unwrap();
        let pty = Master(File::from(master));
        let size = PtySize {
            rows: 40,
            cols: 120,
        };
        pty.resize(size).unwrap();
        let got = rustix::termios::tcgetwinsize(&pty.0).unwrap();
        assert_eq!((got.ws_row, got.ws_col), (40, 120));
    }
}
