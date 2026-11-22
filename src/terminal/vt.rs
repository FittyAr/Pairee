//! The screen of a terminal emulated inside Pairee (the `vt100` crate):
//! what a program running on a PTY (local or on an SSH server) draws, with
//! its colors, cursor and scrollback, so full-screen programs (`vim`,
//! `top`, a `sudo su` shell) work inside the layout instead of taking the
//! whole terminal.
//!
//! The thread reading the program feeds it; the UI paints it and fits it
//! to the area it has. Clones share the screen.

use super::pty::PtySize;
use crate::lock::LockExt;
use std::num::NonZeroU16;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

#[derive(Clone)]
pub struct VtScreen(Arc<Inner>);

struct Inner {
    parser: Mutex<vt100::Parser>,
    /// Bumped by every change, so the UI knows when to repaint.
    version: AtomicU64,
}

impl std::fmt::Debug for VtScreen {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (rows, cols) = self.size();
        f.debug_struct("VtScreen")
            .field("rows", &rows)
            .field("cols", &cols)
            .finish()
    }
}

/// Device status report: "where is the cursor?".
const CURSOR_QUERY: &[u8] = b"\x1b[6n";

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

fn non_zero(n: u16) -> NonZeroU16 {
    NonZeroU16::new(n).unwrap_or(NonZeroU16::MIN)
}

impl VtScreen {
    /// An empty screen of `size` keeping `scrollback` rows that scrolled
    /// off its top.
    pub fn new(size: PtySize, scrollback: usize) -> Self {
        let parser = vt100::Parser::new(non_zero(size.rows), non_zero(size.cols), scrollback);
        Self(Arc::new(Inner {
            parser: Mutex::new(parser),
            version: AtomicU64::new(0),
        }))
    }

    fn parser(&self) -> MutexGuard<'_, vt100::Parser> {
        self.0.parser.lock_safe()
    }

    /// Applies what the program wrote. Returns what a terminal answers to
    /// it: the cursor position for each `ESC [ 6 n` query (ConPTY and
    /// PowerShell wait for it before drawing anything).
    pub fn feed(&self, bytes: &[u8]) -> Vec<u8> {
        let mut answer = Vec::new();
        let mut parser = self.parser();
        let mut rest = bytes;
        while let Some(at) = find(rest, CURSOR_QUERY) {
            let end = at + CURSOR_QUERY.len();
            parser.process(&rest[..end]);
            let (row, col) = parser.screen().cursor_position();
            answer.extend_from_slice(format!("\x1b[{};{}R", row + 1, col + 1).as_bytes());
            rest = &rest[end..];
        }
        parser.process(rest);
        drop(parser);
        self.0.version.fetch_add(1, Ordering::SeqCst);
        answer
    }

    pub fn version(&self) -> u64 {
        self.0.version.load(Ordering::SeqCst)
    }

    /// (rows, cols).
    pub fn size(&self) -> (u16, u16) {
        let (rows, cols) = self.parser().screen().size();
        (rows.get(), cols.get())
    }

    /// Resizes the screen to `size`; `true` when it changed (the program
    /// must then be told).
    pub fn fit(&self, size: PtySize) -> bool {
        if self.size() == (size.rows.max(1), size.cols.max(1)) {
            return false;
        }
        self.parser()
            .set_size(non_zero(size.rows), non_zero(size.cols));
        self.0.version.fetch_add(1, Ordering::SeqCst);
        true
    }

    /// Rows kept above the screen.
    pub fn scrollback_len(&self) -> usize {
        self.parser().screen().scrollback_len()
    }

    /// The program asked for application cursor keys (`ESC O A` instead
    /// of `ESC [ A`), as `vim` and `less` do.
    pub fn application_cursor(&self) -> bool {
        self.parser().screen().application_cursor()
    }

    /// The program enabled bracketed paste.
    pub fn bracketed_paste(&self) -> bool {
        self.parser().screen().bracketed_paste()
    }

    /// Calls `paint` with the screen scrolled `scroll` rows back (0 = the
    /// live screen).
    pub fn with_screen<T>(&self, scroll: usize, paint: impl FnOnce(&vt100::Screen) -> T) -> T {
        let mut parser = self.parser();
        parser.screen_mut().set_scrollback(scroll);
        let result = paint(parser.screen());
        parser.screen_mut().set_scrollback(0);
        result
    }
}

/// `bytes` typed for a key, adjusted to the cursor key mode the program
/// asked for: in application mode the arrows, Home and End send `ESC O x`.
pub fn cursor_keys(bytes: Vec<u8>, application: bool) -> Vec<u8> {
    match (application, bytes.as_slice()) {
        (true, [0x1b, b'[', key @ (b'A' | b'B' | b'C' | b'D' | b'H' | b'F')]) => {
            vec![0x1b, b'O', *key]
        }
        _ => bytes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_lands_on_the_screen_and_fits_the_area() {
        let vt = VtScreen::new(PtySize { rows: 3, cols: 10 }, 100);
        assert!(vt.feed(b"one\r\ntwo\r\nthree\r\nfour").is_empty());
        let top = vt.with_screen(0, |s| s.contents());
        assert_eq!(top, "two\nthree\nfour");
        assert_eq!(vt.scrollback_len(), 1);
        let back = vt.with_screen(1, |s| s.contents());
        assert!(back.starts_with("one"), "{back}");
        assert!(vt.fit(PtySize { rows: 5, cols: 20 }));
        assert!(!vt.fit(PtySize { rows: 5, cols: 20 }));
        assert_eq!(vt.size(), (5, 20));
    }

    #[test]
    fn arrows_follow_the_cursor_key_mode() {
        assert_eq!(cursor_keys(b"\x1b[A".to_vec(), true), b"\x1bOA");
        assert_eq!(cursor_keys(b"\x1b[A".to_vec(), false), b"\x1b[A");
        assert_eq!(cursor_keys(b"x".to_vec(), true), b"x");
        let vt = VtScreen::new(PtySize::DEFAULT, 0);
        assert!(!vt.application_cursor());
        vt.feed(b"\x1b[?1h");
        assert!(vt.application_cursor());
    }

    #[test]
    fn cursor_queries_are_answered_with_the_position() {
        let vt = VtScreen::new(PtySize::DEFAULT, 0);
        assert_eq!(vt.feed(b"ab\x1b[6n"), b"\x1b[1;3R");
        assert_eq!(vt.feed(b"\r\nx\x1b[6ny\x1b[6n"), b"\x1b[2;2R\x1b[2;3R");
    }
}
