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
    let dir = tempfile::Builder::new().prefix("pairee-admin-").tempdir()?;
    let script = dir.path().join(name);
    std::fs::write(&script, body)
        .map_err(|e| anyhow::anyhow!("Failed to write admin-listing script: {}", e))?;
    Ok((dir, script))
}

/// One entry of the helper's output: `name|size|<kind>|<time>` with
/// `fields` fields after the name (a name may itself hold `|`).
fn split_line(line: &str, fields: usize) -> Option<(&str, Vec<&str>)> {
    let mut parts: Vec<&str> = line.rsplitn(fields + 1, '|').collect();
    if parts.len() != fields + 1 {
        return None;
    }
    let name = parts.pop()?;
    parts.reverse();
    Some((name, parts))
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
    let status = crate::fs::privileges::run_powershell(
        "Start-Process powershell -ArgumentList @('-NoProfile','-ExecutionPolicy','Bypass','-File',('\"' + $v0 + '\"'),'-Path',('\"' + $v1 + '\"'),'-OutputFile',('\"' + $v2 + '\"')) -Verb RunAs -WindowStyle Hidden -Wait",
        &[script.as_os_str(), &listed, temp_file.as_os_str()],
    )?;

    if !(status.success() && temp_file.exists()) {
        anyhow::bail!("Failed to read directory as admin")
    }
    let content = std::fs::read_to_string(&temp_file)?;
    let mut entries = Vec::new();
    for line in content.lines() {
        let Some((name, fields)) = split_line(line, 3) else {
            continue;
        };
        let size: u64 = fields[0].parse().unwrap_or(0);
        let mode = fields[1];
        let ticks: i64 = fields[2].parse().unwrap_or(0);
        // .NET ticks (100 ns since 0001-01-01) to Unix time.
        let unix_ticks = ticks - 621_355_968_000_000_000i64;
        let modified = (ticks > 0 && unix_ticks > 0).then(|| {
            SystemTime::UNIX_EPOCH + Duration::from_secs((unix_ticks / 10_000_000) as u64)
        });
        entries.push(FileEntry {
            name: name.to_string(),
            path: path.join(name),
            size,
            is_dir: mode.contains('d') || mode.contains('D'),
            is_symlink: mode.contains('l') || mode.contains('L'),
            modified,
        });
    }
    Ok(entries)
}

#[cfg(not(target_os = "windows"))]
pub(super) fn read_directory_as_admin(path: &Path) -> Result<Vec<FileEntry>> {
    use std::process::Command;

    // Static script: the folder arrives as `sys.argv[1]` and is never
    // parsed as Python source.
    let script_body = r#"import os, sys
path = sys.argv[1]
for e in os.scandir(path):
    try:
        st = e.stat(follow_symlinks=False)
    except OSError:
        continue
    is_dir = 1 if e.is_dir(follow_symlinks=False) else 0
    is_sym = 1 if e.is_symlink() else 0
    print(f"{e.name}|{st.st_size}|{is_dir}|{is_sym}|{int(st.st_mtime)}")
"#;
    let (_private, script) = private_script("ListDirectory.py", script_body)?;

    let output = Command::new("sudo")
        .arg("python3")
        .arg("-B")
        .arg(&script)
        .arg(path)
        .output()?;
    if !output.status.success() {
        anyhow::bail!("Failed to read directory as admin via sudo")
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut entries = Vec::new();
    for line in stdout.lines() {
        let Some((name, fields)) = split_line(line, 4) else {
            continue;
        };
        let mtime: u64 = fields[3].parse().unwrap_or(0);
        entries.push(FileEntry {
            name: name.to_string(),
            path: path.join(name),
            size: fields[0].parse().unwrap_or(0),
            is_dir: fields[1] == "1",
            is_symlink: fields[2] == "1",
            modified: (mtime > 0).then(|| SystemTime::UNIX_EPOCH + Duration::from_secs(mtime)),
        });
    }
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_may_hold_the_separator() {
        let (name, fields) = split_line("a|b.txt|12|0|0|99", 4).unwrap();
        assert_eq!(name, "a|b.txt");
        assert_eq!(fields, ["12", "0", "0", "99"]);
        assert!(split_line("short|1", 4).is_none());
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
