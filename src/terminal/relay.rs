//! Puts a command running on another machine's terminal (see
//! [`super::pty::RemotePty`]) on Pairee's own terminal while the TUI is
//! suspended: its output is written as it comes, and every key, paste and
//! resize goes to it, so interactive programs (`vim`, `top`, `sudo`
//! asking for a password) work as they would over `ssh`.

use super::keys::key_bytes;
use super::pty::{PtySize, RemotePty};
use crossterm::event::{self, Event, KeyEventKind};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use std::io::{self, Read, Write};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

/// How often the keyboard loop looks whether the command ended.
const POLL: Duration = Duration::from_millis(50);

/// Relays `pty` until the command ends; its exit code. Call it with the
/// TUI suspended (cooked mode, main screen): raw mode is on only meanwhile.
pub fn relay(pty: RemotePty) -> io::Result<i32> {
    let RemotePty {
        mut reader,
        mut writer,
        master,
        killer: _killer,
        exit,
    } = pty;
    let ended = Arc::new(AtomicBool::new(false));
    let output = {
        let ended = Arc::clone(&ended);
        std::thread::spawn(move || {
            let mut stdout = io::stdout();
            let mut buf = [0u8; 16 * 1024];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        if stdout
                            .write_all(&buf[..n])
                            .and_then(|()| stdout.flush())
                            .is_err()
                        {
                            break;
                        }
                    }
                }
            }
            ended.store(true, Ordering::SeqCst);
        })
    };

    enable_raw_mode()?;
    let typed = forward_input(&ended, writer.as_mut(), master.as_ref());
    let _ = disable_raw_mode();
    let _ = output.join();
    typed?;
    exit()
}

/// Sends keys, pastes and resizes to the command until its output ends.
fn forward_input(
    ended: &AtomicBool,
    writer: &mut dyn Write,
    master: &dyn super::pty::PtyMaster,
) -> io::Result<()> {
    while !ended.load(Ordering::SeqCst) {
        if !event::poll(POLL)? {
            continue;
        }
        let bytes = match event::read()? {
            Event::Key(key) if key.kind != KeyEventKind::Release => key_bytes(key),
            Event::Paste(text) => Some(text.into_bytes()),
            Event::Resize(cols, rows) => {
                let _ = master.resize(PtySize { rows, cols });
                None
            }
            _ => None,
        };
        if let Some(bytes) = bytes {
            // The command may have just ended: its input is gone then.
            let _ = writer.write_all(&bytes);
        }
    }
    Ok(())
}
