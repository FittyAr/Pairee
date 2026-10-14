//! POSIX signal handling via `signal-hook` on Unix platforms.
//!
//! Listens for termination signals (`SIGINT`, `SIGTERM`, `SIGHUP`,
//! `SIGQUIT`) and turns each into [`Event::Terminate`], so the terminal
//! state (raw mode, alternate screen, mouse capture) is restored and the
//! session saved before the process exits.
//!
//! Window resizes are not handled here: crossterm already reports
//! `SIGWINCH` as a resize event, and a second one would only repaint twice.

#![cfg(unix)]

use crate::terminal::Event;
use signal_hook::consts::signal::*;
use signal_hook::iterator::Signals;
use std::io;
use std::thread;
use tokio::sync::mpsc;

/// Spawns a background thread listening for OS signals and routing them to `sender`.
///
/// The thread keeps listening after the first termination request (a
/// second `SIGTERM` during shutdown must not fall back to an abrupt
/// default action), until the event loop is gone.
pub fn spawn_signal_listener(sender: mpsc::Sender<Event>) -> io::Result<()> {
    let mut signals = Signals::new([SIGINT, SIGTERM, SIGHUP, SIGQUIT])?;

    thread::Builder::new()
        .name("pairee-signal-listener".to_string())
        .spawn(move || {
            for sig in signals.forever() {
                // Ctrl+C / Ctrl+backslash typed for a child running in the
                // terminal reach Pairee too: they are the child's.
                if matches!(sig, SIGINT | SIGQUIT) && super::suspend::child_owns_terminal() {
                    continue;
                }
                tracing::info!(
                    "Received POSIX signal {}, requesting graceful shutdown",
                    sig
                );
                if sender.blocking_send(Event::Terminate).is_err() {
                    // The event loop ended: nobody left to shut down.
                    break;
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

    #[test]
    fn keeps_listening_after_the_first_signal() {
        let (tx, mut rx) = mpsc::channel(10);
        spawn_signal_listener(tx).unwrap();
        for _ in 0..2 {
            signal_hook::low_level::raise(SIGHUP).unwrap();
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
            loop {
                match rx.try_recv() {
                    Ok(Event::Terminate) => break,
                    Ok(other) => panic!("unexpected {other:?}"),
                    Err(_) => {
                        assert!(std::time::Instant::now() < deadline, "signal not seen");
                        std::thread::sleep(std::time::Duration::from_millis(10));
                    }
                }
            }
        }
    }
}
