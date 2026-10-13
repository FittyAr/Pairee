//! A ConPTY pseudo console and the two pipes Pairee talks to it through.

use super::super::{PtyMaster, PtySize};
use std::fs::File;
use std::io;
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use std::ptr;
use windows_sys::Win32::Foundation::S_OK;
use windows_sys::Win32::System::Console::{
    COORD, ClosePseudoConsole, CreatePseudoConsole, HPCON, ResizePseudoConsole,
};
use windows_sys::Win32::System::Pipes::CreatePipe;

/// `PSEUDOCONSOLE_RESIZE_QUIRK | PSEUDOCONSOLE_WIN32_INPUT_MODE`, the flags
/// Pairee always used. Never `PSEUDOCONSOLE_INHERIT_CURSOR`: ConPTY then
/// waits for a cursor report and simple commands can hang (wezterm#6783).
const FLAGS: u32 = 0x2 | 0x4;

/// Pairee's ends of the console: the child's keyboard and its screen.
pub(super) struct ConsolePipes {
    pub input: File,
    pub output: File,
}

/// Closed on drop, which ends the output stream once the child has exited.
pub(super) struct PseudoConsole(HPCON);

impl PseudoConsole {
    pub fn open(size: PtySize) -> io::Result<(Self, ConsolePipes)> {
        let (input_read, input_write) = pipe()?;
        let (output_read, output_write) = pipe()?;
        let mut console: HPCON = 0;
        // SAFETY: both handles are open pipe ends owned by this function and
        // `console` is a valid out pointer. ConPTY duplicates the handles,
        // so ours may close when this function returns.
        let hr = unsafe {
            CreatePseudoConsole(
                coord(size)?,
                input_read.as_raw_handle(),
                output_write.as_raw_handle(),
                FLAGS,
                &mut console,
            )
        };
        check(hr, "CreatePseudoConsole")?;
        let pipes = ConsolePipes {
            input: File::from(input_write),
            output: File::from(output_read),
        };
        Ok((Self(console), pipes))
    }

    pub fn handle(&self) -> HPCON {
        self.0
    }
}

impl PtyMaster for PseudoConsole {
    fn resize(&self, size: PtySize) -> io::Result<()> {
        // SAFETY: `self.0` is a live pseudo console until `drop`.
        check(
            unsafe { ResizePseudoConsole(self.0, coord(size)?) },
            "ResizePseudoConsole",
        )
    }
}

impl Drop for PseudoConsole {
    fn drop(&mut self) {
        // SAFETY: `self.0` came from `CreatePseudoConsole` and is closed
        // exactly once, here.
        unsafe { ClosePseudoConsole(self.0) };
    }
}

/// An anonymous pipe whose ends no child inherits: `(read, write)`.
fn pipe() -> io::Result<(OwnedHandle, OwnedHandle)> {
    let mut read = ptr::null_mut();
    let mut write = ptr::null_mut();
    // SAFETY: valid out pointers, default (non-inheritable) security and
    // buffer size.
    if unsafe { CreatePipe(&mut read, &mut write, ptr::null(), 0) } == 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: `CreatePipe` succeeded, so both are fresh handles nothing
    // else owns.
    Ok(unsafe {
        (
            OwnedHandle::from_raw_handle(read),
            OwnedHandle::from_raw_handle(write),
        )
    })
}

fn coord(size: PtySize) -> io::Result<COORD> {
    let cell = |n: u16| i16::try_from(n).map_err(|_| io::Error::from(io::ErrorKind::InvalidInput));
    Ok(COORD {
        X: cell(size.cols)?,
        Y: cell(size.rows)?,
    })
}

fn check(hr: i32, call: &str) -> io::Result<()> {
    if hr == S_OK {
        Ok(())
    } else {
        Err(io::Error::other(format!(
            "{call} failed: HRESULT {hr:#010x}"
        )))
    }
}
