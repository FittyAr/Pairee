//! Windows PTY: a ConPTY pseudo console fed through two anonymous pipes,
//! with the child started on it by `CreateProcessW`.

mod conpty;
mod launch;

use super::{ChildKiller, ChildWaiter, PtyCommand, SpawnedPty};
use launch::Process;
use std::io;
use std::os::windows::io::AsRawHandle;
use std::sync::Arc;
use windows_sys::Win32::Foundation::WAIT_FAILED;
use windows_sys::Win32::System::Threading::{
    GetExitCodeProcess, INFINITE, TerminateProcess, WaitForSingleObject,
};

/// Exit code of a child stopped by [`Killer`].
const KILLED_EXIT_CODE: u32 = 1;

pub(super) fn spawn(cmd: &PtyCommand) -> io::Result<SpawnedPty> {
    let (console, pipes) = conpty::PseudoConsole::open(cmd.size)?;
    let process = Arc::new(launch::start(cmd, console.handle())?);
    Ok(SpawnedPty {
        pid: Some(process.pid),
        reader: Box::new(pipes.output),
        writer: Box::new(pipes.input),
        master: Box::new(console),
        killer: Box::new(Killer(Arc::clone(&process))),
        waiter: Box::new(Waiter(process)),
    })
}

struct Killer(Arc<Process>);

impl ChildKiller for Killer {
    fn kill(&mut self) -> io::Result<()> {
        // SAFETY: the process handle stays open while `self` holds it.
        if unsafe { TerminateProcess(self.0.handle.as_raw_handle(), KILLED_EXIT_CODE) } == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }
}

struct Waiter(Arc<Process>);

impl ChildWaiter for Waiter {
    fn wait(self: Box<Self>) -> io::Result<bool> {
        let handle = self.0.handle.as_raw_handle();
        // SAFETY: the process handle stays open while `self` holds it.
        if unsafe { WaitForSingleObject(handle, INFINITE) } == WAIT_FAILED {
            return Err(io::Error::last_os_error());
        }
        let mut code = 0u32;
        // SAFETY: as above; `code` is a valid out pointer.
        if unsafe { GetExitCodeProcess(handle, &mut code) } == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(code == 0)
    }
}

#[cfg(test)]
mod tests {
    use super::super::PtySize;
    use super::*;
    use std::io::Read;

    #[test]
    fn child_output_reaches_the_reader_and_exit_code_the_waiter() {
        let cmd = PtyCommand {
            program: std::env::var("ComSpec").unwrap_or_else(|_| "cmd.exe".into()),
            args: ["/D", "/C", "echo %PAIREE_PTY_TEST%& exit 3"]
                .map(String::from)
                .to_vec(),
            env: vec![("PAIREE_PTY_TEST".into(), "pty-on".into())],
            cwd: None,
            size: PtySize::DEFAULT,
        };
        let pty = spawn(&cmd).expect("spawn on ConPTY");
        let SpawnedPty {
            mut reader,
            writer,
            master,
            waiter,
            ..
        } = pty;
        let read = std::thread::spawn(move || {
            let mut out = Vec::new();
            let _ = reader.read_to_end(&mut out);
            String::from_utf8_lossy(&out).into_owned()
        });
        master
            .resize(PtySize {
                rows: 30,
                cols: 100,
            })
            .expect("resize");
        assert!(!waiter.wait().expect("wait"), "exit 3 is a failure");
        // Closing the console ends the output stream.
        drop((writer, master));
        let out = read.join().unwrap();
        assert!(out.contains("pty-on"), "{out:?}");
    }
}
