//! Commands run on a terminal of the SSH server (the command line of an
//! SFTP panel): `vim`, `top`, `sudo` asking for its password and `command &`
//! screens work as they do locally.
//!
//! Each command gets a session of its own, opened with the saved login:
//! a terminal waits for input for as long as the user likes, and libssh2
//! holds a session's lock while it blocks, which would stall the panel's
//! SFTP requests. That session runs non-blocking on one thread (the pump)
//! that does all of its I/O: the parts handed out ([`RemotePty`]) only
//! queue input, resizes and the kill for it, and read the output it
//! forwards.

use super::SharedSshClient;
use crate::terminal::pty::{ChildKiller, PtyMaster, PtySize, RemotePty};
use std::io::{self, Read, Write};
use std::path::Path;
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError, sync_channel};
use std::time::Duration;

/// Terminal type announced to the server.
const TERM: &str = "xterm-256color";
/// Pause of the pump when nothing moved (typing stays instant: input
/// wakes it within this).
const IDLE: Duration = Duration::from_millis(8);
/// libssh2's "would block" (a non-blocking call to try again).
const LIBSSH2_ERROR_EAGAIN: std::ffi::c_int = -37;
/// Exit code reported for a command stopped from Pairee.
const KILLED: i32 = -1;

/// What the parts of a [`RemotePty`] ask of the pump.
enum Request {
    Input(Vec<u8>),
    Resize(PtySize),
    Kill,
}

/// Starts `command` in `dir` on a terminal of `size` on the server of
/// `client`. Network I/O (a new login): call it where waiting is fine.
pub(super) fn spawn(
    client: &SharedSshClient,
    dir: &Path,
    command: &str,
    size: PtySize,
) -> io::Result<RemotePty> {
    let session = client.open_session().map_err(io::Error::other)?;
    let mut channel = session.channel_session().map_err(io::Error::from)?;
    let dim = (u32::from(size.cols), u32::from(size.rows), 0, 0);
    channel
        .request_pty(TERM, None, Some(dim))
        .map_err(io::Error::from)?;
    channel
        .exec(&exec_line(dir, command))
        .map_err(io::Error::from)?;
    session.set_blocking(false);

    let (requests, inbox) = mpsc::channel();
    let (output_tx, output_rx) = mpsc::channel();
    let (exit_tx, exit_rx) = sync_channel(1);
    std::thread::Builder::new()
        .name("pairee-ssh-terminal".to_string())
        .spawn(move || {
            let code = pump(&session, &mut channel, &inbox, output_tx);
            let _ = exit_tx.send(code);
        })?;
    Ok(RemotePty {
        reader: Box::new(Output {
            chunks: output_rx,
            chunk: Vec::new(),
            at: 0,
        }),
        writer: Box::new(Input(requests.clone())),
        master: Box::new(Resizer(requests.clone())),
        killer: Box::new(Killer(requests)),
        exit: Box::new(move || {
            exit_rx
                .recv()
                .unwrap_or_else(|_| Err(io::Error::other("ssh terminal thread ended")))
        }),
    })
}

/// The remote command line: `command` run from `dir` (quoted for the
/// server's POSIX shell, `/` separators whatever the local platform).
pub(super) fn exec_line(dir: &Path, command: &str) -> String {
    let dir = dir.to_string_lossy().replace('\\', "/");
    format!("cd {} && {}", crate::shell::quote_posix(&dir), command)
}

/// Moves input to the server and output to `output` until the command
/// ends (or is killed); then its exit code. `output` is dropped first, so
/// the reader has seen everything when the exit code arrives.
fn pump(
    session: &ssh2::Session,
    channel: &mut ssh2::Channel,
    inbox: &Receiver<Request>,
    output: Sender<Vec<u8>>,
) -> io::Result<i32> {
    let mut input: Vec<u8> = Vec::new();
    let mut resize: Option<PtySize> = None;
    let mut buf = [0u8; 16 * 1024];
    loop {
        let mut moved = false;
        loop {
            match inbox.try_recv() {
                Ok(Request::Input(bytes)) => input.extend_from_slice(&bytes),
                Ok(Request::Resize(size)) => resize = Some(size),
                Ok(Request::Kill) => {
                    // Closing the channel hangs up the server's terminal
                    // (SIGHUP); the session ends with this thread.
                    let _ = channel.close();
                    return Ok(KILLED);
                }
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => break,
            }
        }
        if let Some(size) = resize {
            match channel.request_pty_size(size.cols.into(), size.rows.into(), None, None) {
                Err(e) if e.code() == ssh2::ErrorCode::Session(LIBSSH2_ERROR_EAGAIN) => {}
                _ => resize = None,
            }
        }
        if !input.is_empty() {
            match channel.write(&input) {
                Ok(n) => {
                    input.drain(..n);
                    moved |= n > 0;
                }
                Err(e) if would_block(&e) => {}
                Err(e) => return Err(e),
            }
        }
        match channel.read(&mut buf) {
            Ok(0) if channel.eof() => break,
            Ok(0) => {}
            Ok(n) => {
                let _ = output.send(buf[..n].to_vec());
                moved = true;
            }
            Err(e) if would_block(&e) => {}
            Err(e) => return Err(e),
        }
        if !moved {
            std::thread::sleep(IDLE);
        }
    }
    drop(output);
    session.set_blocking(true);
    channel.wait_close().map_err(io::Error::from)?;
    channel.exit_status().map_err(io::Error::from)
}

fn would_block(e: &io::Error) -> bool {
    e.kind() == io::ErrorKind::WouldBlock
}

/// The command's output, as the pump forwards it.
struct Output {
    chunks: Receiver<Vec<u8>>,
    chunk: Vec<u8>,
    at: usize,
}

impl Read for Output {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        while self.at >= self.chunk.len() {
            match self.chunks.recv() {
                Ok(chunk) => {
                    self.chunk = chunk;
                    self.at = 0;
                }
                Err(_) => return Ok(0),
            }
        }
        let n = buf.len().min(self.chunk.len() - self.at);
        buf[..n].copy_from_slice(&self.chunk[self.at..self.at + n]);
        self.at += n;
        Ok(n)
    }
}

/// The command's keyboard.
struct Input(Sender<Request>);

impl Write for Input {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0
            .send(Request::Input(buf.to_vec()))
            .map_err(|_| io::Error::from(io::ErrorKind::BrokenPipe))?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

struct Resizer(Sender<Request>);

impl PtyMaster for Resizer {
    fn resize(&self, size: PtySize) -> io::Result<()> {
        // Once the command ended there is nothing left to resize.
        let _ = self.0.send(Request::Resize(size));
        Ok(())
    }
}

struct Killer(Sender<Request>);

impl ChildKiller for Killer {
    fn kill(&mut self) -> io::Result<()> {
        let _ = self.0.send(Request::Kill);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remote_commands_start_in_the_quoted_panel_folder() {
        assert_eq!(
            exec_line(Path::new("/home/deploy"), "mkdir certs"),
            "cd '/home/deploy' && mkdir certs"
        );
        assert_eq!(
            exec_line(Path::new("/srv/it's; rm -rf ~"), "ls"),
            r"cd '/srv/it'\''s; rm -rf ~' && ls"
        );
        // A path joined on Windows keeps `/` for the server.
        assert_eq!(
            exec_line(Path::new(r"/home\deck\umbral"), "ls"),
            "cd '/home/deck/umbral' && ls"
        );
    }

    #[test]
    fn output_is_read_in_order_and_ends_when_the_pump_does() {
        let (tx, rx) = mpsc::channel();
        let mut out = Output {
            chunks: rx,
            chunk: Vec::new(),
            at: 0,
        };
        tx.send(b"hello ".to_vec()).unwrap();
        tx.send(b"world".to_vec()).unwrap();
        drop(tx);
        let mut text = String::new();
        out.read_to_string(&mut text).unwrap();
        assert_eq!(text, "hello world");
    }
}
