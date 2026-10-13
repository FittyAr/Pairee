//! `CreateProcessW` of a child attached to a pseudo console: the attribute
//! list, the command line and the environment block.

use super::super::PtyCommand;
use std::collections::BTreeMap;
use std::ffi::{OsStr, OsString, c_void};
use std::io;
use std::os::windows::ffi::OsStrExt;
use std::os::windows::io::{FromRawHandle, OwnedHandle};
use std::path::Path;
use std::ptr;
use windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE;
use windows_sys::Win32::System::Console::HPCON;
use windows_sys::Win32::System::Threading::{
    CREATE_UNICODE_ENVIRONMENT, CreateProcessW, DeleteProcThreadAttributeList,
    EXTENDED_STARTUPINFO_PRESENT, InitializeProcThreadAttributeList, LPPROC_THREAD_ATTRIBUTE_LIST,
    PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE, PROCESS_INFORMATION, STARTF_USESTDHANDLES, STARTUPINFOEXW,
    UpdateProcThreadAttribute,
};

/// A started child: its process handle (for waiting and killing) and id.
pub(super) struct Process {
    pub handle: OwnedHandle,
    pub pid: u32,
}

pub(super) fn start(cmd: &PtyCommand, console: HPCON) -> io::Result<Process> {
    let attributes = AttributeList::with_console(console)?;
    let mut startup = STARTUPINFOEXW::default();
    startup.StartupInfo.cb = size_of::<STARTUPINFOEXW>() as u32;
    // No stdio handles: without this a child could inherit Pairee's own
    // redirected stdout instead of writing to the console.
    startup.StartupInfo.dwFlags = STARTF_USESTDHANDLES;
    startup.StartupInfo.hStdInput = INVALID_HANDLE_VALUE;
    startup.StartupInfo.hStdOutput = INVALID_HANDLE_VALUE;
    startup.StartupInfo.hStdError = INVALID_HANDLE_VALUE;
    startup.lpAttributeList = attributes.as_ptr();

    // `None` lets Windows search for a bare name; an absolute path (the
    // usual `%ComSpec%`) is used as is, never looked up in the current dir.
    let application = Path::new(&cmd.program)
        .is_absolute()
        .then(|| wide(OsStr::new(&cmd.program)))
        .transpose()?;
    let mut command_line = wide(OsStr::new(&command_line(cmd)))?;
    let environment = environment_block(std::env::vars_os(), &cmd.env);
    let cwd = cmd
        .cwd
        .as_deref()
        .map(|d| wide(d.as_os_str()))
        .transpose()?;
    let mut info = PROCESS_INFORMATION::default();
    // SAFETY: every pointer is either null or points to a NUL-terminated
    // UTF-16 buffer (the environment block is double-NUL-terminated) that
    // outlives the call; `startup` is a valid STARTUPINFOEXW whose attribute
    // list is initialised; no handle is inherited.
    let ok = unsafe {
        CreateProcessW(
            application.as_ref().map_or(ptr::null(), |a| a.as_ptr()),
            command_line.as_mut_ptr(),
            ptr::null(),
            ptr::null(),
            0,
            EXTENDED_STARTUPINFO_PRESENT | CREATE_UNICODE_ENVIRONMENT,
            environment.as_ptr().cast::<c_void>(),
            cwd.as_ref().map_or(ptr::null(), |d| d.as_ptr()),
            &startup.StartupInfo,
            &mut info,
        )
    };
    if ok == 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: `CreateProcessW` succeeded and returned two fresh handles
    // that only `info` refers to; the thread one is closed right away.
    let (handle, _thread) = unsafe {
        (
            OwnedHandle::from_raw_handle(info.hProcess),
            OwnedHandle::from_raw_handle(info.hThread),
        )
    };
    Ok(Process {
        handle,
        pid: info.dwProcessId,
    })
}

/// A one-entry attribute list naming the pseudo console.
struct AttributeList(Vec<usize>);

impl AttributeList {
    fn with_console(console: HPCON) -> io::Result<Self> {
        let mut bytes = 0usize;
        // SAFETY: a null list with a count is the documented size query; it
        // fails with ERROR_INSUFFICIENT_BUFFER after setting `bytes`.
        unsafe { InitializeProcThreadAttributeList(ptr::null_mut(), 1, 0, &mut bytes) };
        let mut list = Self(vec![0usize; bytes.div_ceil(size_of::<usize>())]);
        // SAFETY: the buffer holds at least `bytes` pointer-aligned bytes.
        if unsafe { InitializeProcThreadAttributeList(list.as_ptr(), 1, 0, &mut bytes) } == 0 {
            list.0 = Vec::new();
            return Err(io::Error::last_os_error());
        }
        // SAFETY: the list is initialised for one attribute; for the pseudo
        // console attribute the value *is* the HPCON (passed as the pointer
        // argument), which outlives the `CreateProcessW` call.
        let ok = unsafe {
            UpdateProcThreadAttribute(
                list.as_ptr(),
                0,
                PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE as usize,
                console as *const c_void,
                size_of::<HPCON>(),
                ptr::null_mut(),
                ptr::null(),
            )
        };
        if ok == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(list)
    }

    fn as_ptr(&self) -> LPPROC_THREAD_ATTRIBUTE_LIST {
        self.0.as_ptr().cast_mut().cast()
    }
}

impl Drop for AttributeList {
    fn drop(&mut self) {
        if !self.0.is_empty() {
            // SAFETY: a non-empty buffer is an initialised list, deleted
            // once.
            unsafe { DeleteProcThreadAttributeList(self.as_ptr()) };
        }
    }
}

/// Program and arguments joined with the `CommandLineToArgvW` rules; an
/// argument is quoted only when it needs it (cmd.exe does not recognise a
/// quoted `"/C"` switch).
fn command_line(cmd: &PtyCommand) -> String {
    std::iter::once(&cmd.program)
        .chain(&cmd.args)
        .map(|arg| {
            if arg.is_empty() || arg.contains([' ', '\t', '"']) {
                crate::shell::quote_msvcrt(arg)
            } else {
                arg.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// `inherited` plus `extra` (which wins), names compared case-insensitively
/// and sorted as Windows expects, as a `NAME=value\0...\0` block.
fn environment_block(
    inherited: impl Iterator<Item = (OsString, OsString)>,
    extra: &[(String, String)],
) -> Vec<u16> {
    let extra = extra.iter().map(|(k, v)| (k.into(), v.into()));
    let vars: BTreeMap<String, (OsString, OsString)> = inherited
        .chain(extra)
        .map(|(k, v): (OsString, OsString)| (k.to_string_lossy().to_uppercase(), (k, v)))
        .collect();
    let mut block = Vec::new();
    for (name, value) in vars.values() {
        block.extend(name.encode_wide());
        block.push(u16::from(b'='));
        block.extend(value.encode_wide());
        block.push(0);
    }
    if block.is_empty() {
        block.push(0);
    }
    block.push(0);
    block
}

/// NUL-terminated UTF-16; an embedded NUL would silently cut the string.
fn wide(s: &OsStr) -> io::Result<Vec<u16>> {
    let mut out: Vec<u16> = s.encode_wide().collect();
    if out.contains(&0) {
        return Err(io::Error::from(io::ErrorKind::InvalidInput));
    }
    out.push(0);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terminal::pty::PtySize;

    fn command(program: &str, args: &[&str]) -> PtyCommand {
        PtyCommand {
            program: program.into(),
            args: args.iter().map(|a| (*a).to_string()).collect(),
            env: Vec::new(),
            cwd: None,
            size: PtySize::DEFAULT,
        }
    }

    #[test]
    fn command_line_quotes_only_what_needs_it() {
        let cmd = command(r"C:\Program Files\x.exe", &["/C", "%V%", "a b", "", "q\"t"]);
        assert_eq!(
            command_line(&cmd),
            r#""C:\Program Files\x.exe" /C %V% "a b" "" "q\"t""#
        );
    }

    #[test]
    fn environment_block_merges_case_insensitively_and_sorts() {
        let inherited = [("Path", "a"), ("b", "1")].map(|(k, v)| (k.into(), v.into()));
        let block = environment_block(inherited.into_iter(), &[("PATH".into(), "z".into())]);
        let text = String::from_utf16(&block).unwrap();
        assert_eq!(text, "b=1\0PATH=z\0\0");
    }

    #[test]
    fn empty_environment_block_is_two_nuls() {
        assert_eq!(environment_block(std::iter::empty(), &[]), vec![0, 0]);
    }

    #[test]
    fn wide_rejects_embedded_nul() {
        assert!(wide(OsStr::new("a\0b")).is_err());
        assert_eq!(wide(OsStr::new("a")).unwrap(), vec![u16::from(b'a'), 0]);
    }
}
