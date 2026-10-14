//! Listing a folder the user cannot read, through an elevated helper
//! (UAC on Windows, `sudo` elsewhere). The helper runs a static script
//! that gets the folder as a parameter, never as code.

use crate::fs::entry::FileEntry;
use anyhow::Result;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

/// Writes `body` as `name` in a new private temporary folder (0700 on
/// Unix) and returns both: the elevated helper runs exactly this script.
/// A fixed script in the shared temp folder could be planted beforehand by
/// another user and would then run as root / Administrator.
fn private_script(name: &str, body: &str) -> Result<(tempfile::TempDir, PathBuf)> {
    let dir = crate::fs::privileges::private_temp_dir("pairee-admin-")?;
    let script = dir.path().join(name);
    std::fs::write(&script, body)
        .map_err(|e| anyhow::anyhow!("Failed to write admin-listing script: {}", e))?;
    Ok((dir, script))
}

/// One entry of the helper's output: `name|size|<kind>|<time>` with
/// `fields` fields after the name (a name may itself hold `|`). Bytes, not
/// text: a Unix name need not be UTF-8.
fn split_line(line: &[u8], fields: usize) -> Option<(&[u8], Vec<&[u8]>)> {
    let mut parts: Vec<&[u8]> = line.rsplitn(fields + 1, |&b| b == b'|').collect();
    if parts.len() != fields + 1 {
        return None;
    }
    let name = parts.pop()?;
    parts.reverse();
    Some((name, parts))
}

/// A numeric field of the helper's output (`default` when unreadable).
fn number<T: std::str::FromStr + Default>(field: &[u8]) -> T {
    std::str::from_utf8(field)
        .ok()
        .and_then(|text| text.trim().parse().ok())
        .unwrap_or_default()
}

#[cfg(target_os = "windows")]
pub(super) fn read_directory_as_admin(path: &Path) -> Result<Vec<FileEntry>> {
    // Static script: the folder and the output file arrive as parameters.
    let script_body = r#"param(
    [Parameter(Mandatory = $true, Position = 0)]
    [string] $Path,
    [Parameter(Mandatory = $true, Position = 1)]
    [string] $OutputFile
)

$ErrorActionPreference = 'Stop'
$utf8NoBom = New-Object System.Text.UTF8Encoding($false)
$writer = New-Object System.IO.StreamWriter($OutputFile, $false, $utf8NoBom)

try {
    Get-ChildItem -LiteralPath $Path -Force -ErrorAction Stop | ForEach-Object {
        $name = $_.Name
        $length = $_.Length
        $mode = $_.Mode
        $ticks = $_.LastWriteTime.Ticks
        $writer.WriteLine(("{0}|{1}|{2}|{3}" -f $name, $length, $mode, $ticks))
    }
} finally {
    $writer.Close()
}
"#;
    let (private, script) = private_script("ListDirectory.ps1", script_body)?;
    let temp_file = private.path().join("listing.txt");
    // A folder ending in `\` would escape the closing quote of its argument
    // on the helper's command line: `C:\` is passed as `C:\.`.
    let mut listed = path.as_os_str().to_owned();
    if listed.to_string_lossy().ends_with('\\') {
        listed.push(".");
    }
    // `Start-Process` joins the arguments with spaces: each path is quoted
    // for the helper's command line, and `-File` passes them as plain text.
    let status = crate::fs::privileges::powershell(
        "Start-Process powershell -ArgumentList @('-NoProfile','-ExecutionPolicy','Bypass','-File',('\"' + $v0 + '\"'),'-Path',('\"' + $v1 + '\"'),'-OutputFile',('\"' + $v2 + '\"')) -Verb RunAs -WindowStyle Hidden -Wait",
        &[script.as_os_str(), &listed, temp_file.as_os_str()],
    )?
    .status()?;

    if !(status.success() && temp_file.exists()) {
        anyhow::bail!("Failed to read directory as admin")
    }
    let content = std::fs::read_to_string(&temp_file)?;
    let mut entries = Vec::new();
    for line in content.lines() {
        let Some((name, fields)) = split_line(line.as_bytes(), 3) else {
            continue;
        };
        let name = String::from_utf8_lossy(name);
        let size: u64 = number(fields[0]);
        let mode = String::from_utf8_lossy(fields[1]);
        let ticks: i64 = number(fields[2]);
        // .NET ticks (100 ns since 0001-01-01) to Unix time.
        let unix_ticks = ticks - 621_355_968_000_000_000i64;
        let modified = (ticks > 0 && unix_ticks > 0).then(|| {
            SystemTime::UNIX_EPOCH + Duration::from_secs((unix_ticks / 10_000_000) as u64)
        });
        entries.push(FileEntry {
            name: name.to_string(),
            path: path.join(&*name),
            size,
            is_dir: mode.contains('d') || mode.contains('D'),
            is_symlink: mode.contains('l') || mode.contains('L'),
            modified,
            ..Default::default()
        });
    }
    Ok(entries)
}

#[cfg(not(target_os = "windows"))]
pub(super) fn read_directory_as_admin(path: &Path) -> Result<Vec<FileEntry>> {
    use std::os::unix::ffi::OsStrExt;

    // Static script: the folder and the output file arrive as `sys.argv`
    // and are never parsed as Python source. Names go out as the raw bytes
    // the filesystem has (`print` would die on a name that is not UTF-8);
    // a name with a line break cannot be told apart in the output and is
    // left out.
    let script_body = r#"import os, sys
path, out = sys.argv[1], sys.argv[2]
fd = os.open(out, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o644)
with os.fdopen(fd, "wb") as f:
    for e in os.scandir(path):
        name = os.fsencode(e.name)
        if b"\n" in name or b"\r" in name:
            continue
        try:
            st = e.stat(follow_symlinks=False)
        except OSError:
            continue
        is_dir = 1 if e.is_dir(follow_symlinks=False) else 0
        is_sym = 1 if e.is_symlink() else 0
        f.write(name + b"|%d|%d|%d|%d\n" % (st.st_size, is_dir, is_sym, int(st.st_mtime)))
"#;
    let (private, script) = private_script("ListDirectory.py", script_body)?;
    let listing = private.path().join("listing.txt");

    // On the user's terminal, TUI suspended: `sudo` asks for its password
    // there (captured pipes and a null stdin leave it nowhere to ask, and a
    // prompt under the running TUI would eat the keys). The listing comes
    // back through a file, as on Windows.
    let status = crate::fs::privileges::run_sudo(
        "Requesting administrator privileges to list the folder...",
        &[
            std::ffi::OsStr::new("python3"),
            std::ffi::OsStr::new("-B"),
            script.as_os_str(),
            path.as_os_str(),
            listing.as_os_str(),
        ],
    )?;
    if !(status.success() && listing.exists()) {
        anyhow::bail!("Failed to read directory as admin via sudo")
    }
    let stdout = std::fs::read(&listing)?;
    let mut entries = Vec::new();
    for line in stdout.split(|&b| b == b'\n') {
        let Some((name, fields)) = split_line(line, 4) else {
            continue;
        };
        let mtime: u64 = number(fields[3]);
        let os_name = std::ffi::OsStr::from_bytes(name);
        entries.push(FileEntry {
            name: os_name.to_string_lossy().into_owned(),
            path: path.join(os_name),
            size: number(fields[0]),
            is_dir: fields[1] == b"1",
            is_symlink: fields[2] == b"1",
            modified: (mtime > 0).then(|| SystemTime::UNIX_EPOCH + Duration::from_secs(mtime)),
            ..Default::default()
        });
    }
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_may_hold_the_separator() {
        let (name, fields) = split_line(b"a|b.txt|12|0|0|99", 4).unwrap();
        assert_eq!(name, b"a|b.txt");
        assert_eq!(fields, [&b"12"[..], b"0", b"0", b"99"]);
        assert!(split_line(b"short|1", 4).is_none());
    }

    /// A name that is not UTF-8 survives the split, byte for byte.
    #[test]
    fn names_may_be_arbitrary_bytes() {
        let (name, fields) = split_line(b"caf\xe9|3|0|0|7", 4).unwrap();
        assert_eq!(name, b"caf\xe9");
        assert_eq!(number::<u64>(fields[0]), 3);
        assert_eq!(number::<u64>(b"x"), 0);
    }

    #[test]
    fn scripts_live_in_a_private_folder() {
        let (dir, script) = private_script("s.py", "print(1)").unwrap();
        assert!(script.starts_with(dir.path()));
        assert_eq!(std::fs::read_to_string(&script).unwrap(), "print(1)");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(dir.path()).unwrap().permissions().mode();
            assert_eq!(mode & 0o077, 0, "{mode:o}");
        }
    }
}
