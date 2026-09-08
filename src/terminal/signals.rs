//! POSIX signal handling via `signal-hook` on Unix platforms.
//!
//! Listens for termination (`SIGINT`, `SIGTERM`, `SIGHUP`, `SIGQUIT`) and
//! window resize (`SIGWINCH`) signals, converting them into typed [`Event`]s
//! to ensure the terminal state (raw mode, alternate screen, mouse capture)
//! is cleanly restored before process exit.

#![cfg(unix)]

use crate::terminal::Event;
use signal_hook::consts::signal::*;
use signal_hook::iterator::Signals;
use std::io;
use std::thread;
use tokio::sync::mpsc;

/// Spawns a background thread listening for OS signals and routing them to `sender`.
pub fn spawn_signal_listener(sender: mpsc::Sender<Event>) -> io::Result<()> {
    let mut signals = Signals::new([SIGINT, SIGTERM, SIGHUP, SIGQUIT, SIGWINCH])?;

    thread::Builder::new()
        .name("pairee-signal-listener".to_string())
        .spawn(move || {
            for sig in signals.forever() {
                match sig {
                    SIGWINCH => {
                        if let Ok((w, h)) = crossterm::terminal::size()
                            && sender.blocking_send(Event::Resize(w, h)).is_err()
                        {
                            break;
                        }
                    }
                    SIGINT | SIGTERM | SIGHUP | SIGQUIT => {
                        tracing::info!(
                            "Received POSIX signal {}, requesting graceful shutdown",
                            sig
                        );
                        let _ = sender.blocking_send(Event::Terminate);
                        break;
                    }
                    _ => {}
                }
            }
        })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signal_listener_registration() {
        let (tx, _rx) = mpsc::channel(10);
        let res = spawn_signal_listener(tx);
        assert!(res.is_ok());
    }
}
