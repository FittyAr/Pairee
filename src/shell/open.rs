//! Open a file with the handler registered by the operating system, without
//! going through a shell (no `cmd /c start`), so the file name is never
//! parsed as a command line.

use std::path::Path;

/// Open `path` with the system default application, in the background.
/// Failures are logged; there is nothing useful the caller can do with them.
pub fn open_with_system_handler(path: &Path) {
    let path = path.to_path_buf();
    // ShellExecuteW may block (DDE, slow handlers); keep the UI responsive.
    std::thread::spawn(move || {
        if let Err(e) = open_blocking(&path) {
            log::warn!(
                "Cannot open {} with the system handler: {e}",
                path.display()
            );
        }
    });
}

#[cfg(windows)]
fn open_blocking(path: &Path) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::UI::Shell::ShellExecuteW;
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    let wide = |s: &std::ffi::OsStr| -> Vec<u16> { s.encode_wide().chain(Some(0)).collect() };
    let file = wide(path.as_os_str());
    let verb = wide(std::ffi::OsStr::new("open"));
    // SAFETY: both strings are NUL-terminated UTF-16 buffers that outlive
    // the call; null hwnd / parameters / directory are documented as valid.
    let result = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            verb.as_ptr(),
            file.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            SW_SHOWNORMAL,
        )
    };
    // Values <= 32 are error codes (documented HINSTANCE compatibility).
    if result as usize > 32 {
        Ok(())
    } else {
        Err(std::io::Error::other(format!(
            "ShellExecuteW failed with code {}",
            result as usize
        )))
    }
}

#[cfg(not(windows))]
fn open_blocking(path: &Path) -> std::io::Result<()> {
    let opener = if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    // The path is a single argv entry, never a shell string.
    std::process::Command::new(opener)
        .arg(path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map(|_| ())
}
