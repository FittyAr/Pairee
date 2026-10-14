//! Orderly shutdown when Windows ends Pairee.
//!
//! * Console control events (`SetConsoleCtrlHandler`): Ctrl+C / Ctrl+Break,
//!   closing the console window, and logoff / shutdown when they arrive as
//!   console events.
//! * Logoff / shutdown as window messages: a process that loads `user32`
//!   (Pairee does) gets no console event for them, only
//!   `WM_QUERYENDSESSION` / `WM_ENDSESSION` on a top-level window, so a
//!   hidden one listens for them on its own thread.
//!
//! Each sends [`Event::Terminate`] to the event loop. For the ending events
//! Windows terminates the process as soon as the handler returns, so the
//! handler then waits (within the time Windows allows) until
//! [`cleanup_finished`] reports that the session was saved and the
//! terminal restored.

#![cfg(windows)]

use crate::lock::LockExt;
use crate::terminal::Event;
use std::sync::{Condvar, Mutex};
use std::time::Duration;
use tokio::sync::mpsc;
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::System::Console::{
    CTRL_BREAK_EVENT, CTRL_C_EVENT, CTRL_CLOSE_EVENT, CTRL_LOGOFF_EVENT, CTRL_SHUTDOWN_EVENT,
    SetConsoleCtrlHandler,
};

static SENDER: Mutex<Option<mpsc::Sender<Event>>> = Mutex::new(None);
static CLEANUP: CleanupGate = CleanupGate::new();

/// How long an ending event holds the process for the cleanup. Windows
/// kills a console process about 5 s after a close, logoff or shutdown
/// event (and a hung window about as long after `WM_ENDSESSION`).
const CLEANUP_BUDGET: Duration = Duration::from_millis(4500);

/// Opens once the main loop finished its cleanup; waiters time out.
pub struct CleanupGate {
    done: Mutex<bool>,
    changed: Condvar,
}

impl CleanupGate {
    pub const fn new() -> Self {
        Self {
            done: Mutex::new(false),
            changed: Condvar::new(),
        }
    }

    pub fn finish(&self) {
        *self.done.lock_safe() = true;
        self.changed.notify_all();
    }

    /// `true` once finished, `false` when `timeout` passed first.
    pub fn wait(&self, timeout: Duration) -> bool {
        let done = self.done.lock_safe();
        let (done, _) = self
            .changed
            .wait_timeout_while(done, timeout, |done| !*done)
            .unwrap_or_else(|e| e.into_inner());
        *done
    }
}

/// Reports that Pairee saved what it keeps and restored the terminal: an
/// ending event waiting in [`console_ctrl_handler`] or the session-end
/// window lets Windows end the process now.
pub fn cleanup_finished() {
    CLEANUP.finish();
}

/// Asks the event loop to quit; `false` when there is none (yet).
fn request_shutdown() -> bool {
    let sender = SENDER.lock_safe().clone();
    sender.is_some_and(|sender| sender.blocking_send(Event::Terminate).is_ok())
}

/// [`request_shutdown`], then holds the calling (OS) thread until the
/// cleanup finished or the budget is spent.
fn shutdown_and_wait() -> bool {
    if !request_shutdown() {
        return false;
    }
    if !CLEANUP.wait(CLEANUP_BUDGET) {
        log::warn!("Windows is ending Pairee before its cleanup finished");
    }
    true
}

unsafe extern "system" fn console_ctrl_handler(ctrl_type: u32) -> i32 {
    match ctrl_type {
        // Typed for a child running in the console, which receives it too:
        // handled (ignored) here so Pairee keeps running.
        CTRL_C_EVENT | CTRL_BREAK_EVENT if super::suspend::child_owns_terminal() => 1,
        CTRL_C_EVENT | CTRL_BREAK_EVENT => i32::from(request_shutdown()),
        // The process ends when this returns: not before the cleanup.
        CTRL_CLOSE_EVENT | CTRL_LOGOFF_EVENT | CTRL_SHUTDOWN_EVENT => {
            i32::from(shutdown_and_wait())
        }
        _ => 0,
    }
}

/// Registers the Windows console control handler and the session-end
/// listener to gracefully intercept process termination.
pub fn setup_windows_ctrl_handler(sender: mpsc::Sender<Event>) {
    *SENDER.lock_safe() = Some(sender);
    // SAFETY: registers a handler with the signature Windows expects; it
    // only touches thread-safe statics.
    unsafe {
        SetConsoleCtrlHandler(Some(console_ctrl_handler), 1);
    }
    let spawned = std::thread::Builder::new()
        .name("pairee-session-end".to_string())
        .spawn(session_end::listen);
    if let Err(e) = spawned {
        log::warn!("Logoff / shutdown listener not started: {e}");
    }
}

/// The hidden window that hears logoff and shutdown.
mod session_end {
    use super::*;
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, MSG, RegisterClassW,
        TranslateMessage, WM_ENDSESSION, WM_QUERYENDSESSION, WNDCLASSW,
    };

    unsafe extern "system" fn window_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        match msg {
            // Pairee never vetoes the end of the session.
            WM_QUERYENDSESSION => 1,
            // The session really ends (wparam != 0): the process is gone
            // once this returns.
            WM_ENDSESSION => {
                if wparam != 0 {
                    shutdown_and_wait();
                }
                0
            }
            // SAFETY: the default handling of a message of our own window.
            _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
        }
    }

    /// Creates the (never shown) top-level window and pumps its messages
    /// for the life of the process.
    pub(super) fn listen() {
        let class: Vec<u16> = "PaireeSessionEnd\0".encode_utf16().collect();
        // SAFETY: plain Win32 calls with valid pointers: `class` outlives
        // the window (this thread pumps until the process ends), `msg` is
        // a valid out pointer.
        unsafe {
            let instance = GetModuleHandleW(std::ptr::null());
            let wc = WNDCLASSW {
                lpfnWndProc: Some(window_proc),
                hInstance: instance,
                lpszClassName: class.as_ptr(),
                ..std::mem::zeroed()
            };
            if RegisterClassW(&wc) == 0 {
                log::warn!(
                    "Logoff / shutdown listener: {}",
                    std::io::Error::last_os_error()
                );
                return;
            }
            let hwnd = CreateWindowExW(
                0,
                class.as_ptr(),
                std::ptr::null(),
                0,
                0,
                0,
                0,
                0,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                instance,
                std::ptr::null(),
            );
            if hwnd.is_null() {
                log::warn!(
                    "Logoff / shutdown listener: {}",
                    std::io::Error::last_os_error()
                );
                return;
            }
            let mut msg: MSG = std::mem::zeroed();
            while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
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

    #[test]
    fn an_ending_event_waits_for_the_cleanup_within_its_budget() {
        let gate = std::sync::Arc::new(CleanupGate::new());
        assert!(!gate.wait(Duration::from_millis(20)), "times out");
        let opener = std::sync::Arc::clone(&gate);
        let started = std::time::Instant::now();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(50));
            opener.finish();
        });
        assert!(gate.wait(Duration::from_secs(5)));
        assert!(started.elapsed() < Duration::from_secs(4));
        assert!(gate.wait(Duration::ZERO), "stays open");
    }
}
