//! Windows Console control handler.
//!
//! Listens for `CTRL_C_EVENT`, `CTRL_CLOSE_EVENT`, and `CTRL_SHUTDOWN_EVENT` via
//! [`windows_sys::Win32::System::Console::SetConsoleCtrlHandler`], sending [`Event::Terminate`]
//! into the event loop for graceful terminal cleanup before exit.

#![cfg(windows)]

use crate::terminal::Event;
use std::sync::Mutex;
use tokio::sync::mpsc;
use windows_sys::Win32::System::Console::{
    CTRL_C_EVENT, CTRL_CLOSE_EVENT, CTRL_SHUTDOWN_EVENT, SetConsoleCtrlHandler,
};

static SENDER_LOCK: Mutex<Option<mpsc::Sender<Event>>> = Mutex::new(None);

unsafe extern "system" fn console_ctrl_handler(ctrl_type: u32) -> i32 {
    match ctrl_type {
        CTRL_C_EVENT | CTRL_CLOSE_EVENT | CTRL_SHUTDOWN_EVENT => {
            if let Ok(guard) = SENDER_LOCK.lock()
                && let Some(ref sender) = *guard
            {
                let _ = sender.blocking_send(Event::Terminate);
                return 1;
            }
            0
        }
        _ => 0,
    }
}

/// Registers the Windows console control handler to gracefully intercept process termination.
pub fn setup_windows_ctrl_handler(sender: mpsc::Sender<Event>) {
    if let Ok(mut guard) = SENDER_LOCK.lock() {
        *guard = Some(sender);
    }
    unsafe {
        SetConsoleCtrlHandler(Some(console_ctrl_handler), 1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_windows_ctrl_handler_setup() {
        let (tx, _rx) = mpsc::channel(10);
        setup_windows_ctrl_handler(tx);
    }
}
