use crate::fs::entry::FileEntry;
use anyhow::Result;
use std::path::Path;

#[cfg(target_os = "windows")]
pub(super) fn read_directory_as_admin(path: &Path) -> Result<Vec<FileEntry>> {
    use std::io::Write;
    use std::process::Command;

    // The previous implementation built a PowerShell command by string-
    // concatenation of the user-supplied path. The path was escaped for
    // `"` only, leaving it vulnerable to PowerShell injection via `$`,
    // `` ` ``, or `'` in the path. Worse, the inner script was passed as
    // a single-quoted argument to `Start-Process -ArgumentList`, and the
    // inner script's quotes weren't escaped for that context either.
    //
    // The fix: ship a static PowerShell script that takes `-Path` and
    // `-OutputFile` parameters. The path is delivered as a parameter and
    // never re-parsed by PowerShell, so metacharacters are inert.
    let script_dir = std::env::temp_dir().join("pairee-admin");
    std::fs::create_dir_all(&script_dir)
        .map_err(|e| anyhow::anyhow!("Failed to create temp script directory: {}", e))?;
    let script_path = script_dir.join("ListDirectory.ps1");

    // Script body is fully static; no path interpolation.
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
    Get-ChildItem -Path $Path -Force -ErrorAction Stop | ForEach-Object {
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
    if !script_path.exists() {
        std::fs::File::create(&script_path)
            .and_then(|mut f| f.write_all(script_body.as_bytes()))
            .map_err(|e| anyhow::anyhow!("Failed to write admin-listing script: {}", e))?;
    }

    let temp_dir = std::env::temp_dir();
    // UUID rather than the (guessable) process id so a local attacker
    // cannot pre-create the output file with a forged directory listing
    // that the unprivileged parent would then trust.
    let temp_file = temp_dir.join(format!("pairee_dir_{}.txt", uuid::Uuid::new_v4()));
    // We need to pass the *absolute* path to the script because the elevated
    // process will run with a different working directory and we cannot rely
    // on relative paths to resolve to the same location.
    let script_arg = script_path
        .canonicalize()
        .unwrap_or_else(|_| script_path.clone());
    let temp_file_arg = temp_file
        .canonicalize()
        .unwrap_or_else(|_| temp_file.clone());

    // The wrapper that elevates the helper script. We pass the path to the
    // .ps1 and its arguments through `-ArgumentList` (which is parsed by
    // PowerShell at the elevated prompt, NOT as a string), so the path is
    // never re-interpreted.
    let status = Command::new("powershell")
        .arg("-NoProfile")
        .arg("-Command")
        .arg(format!(
            "Start-Process powershell -ArgumentList @('-NoProfile','-ExecutionPolicy','Bypass','-File',\"{}\",'-Path',\"{}\",'-OutputFile',\"{}\") -Verb RunAs -WindowStyle Hidden -Wait",
            script_arg.display(),
            path.display(),
            temp_file_arg.display(),
        ))
        .status()?;

    if status.success() && temp_file.exists() {
        let content = std::fs::read_to_string(&temp_file)?;
        let _ = std::fs::remove_file(&temp_file);
        let mut entries = Vec::new();
        for line in content.lines() {
            let parts: Vec<&str> = line.split('|').collect();
            if parts.len() >= 4 {
                let name = parts[0].to_string();
                let size: u64 = parts[1].parse().unwrap_or(0);
                let mode = parts[2];
                let is_dir = mode.contains('d') || mode.contains('D');
                let ticks: i64 = parts[3].parse().unwrap_or(0);

                let modified = if ticks > 0 {
                    let epoch_ticks = 621355968000000000i64;
                    let unix_ticks = ticks - epoch_ticks;
                    let secs = unix_ticks / 10_000_000;
                    if secs > 0 {
                        Some(
                            std::time::SystemTime::UNIX_EPOCH
                                + std::time::Duration::from_secs(secs as u64),
                        )
                    } else {
                        None
                    }
                } else {
                    None
                };

                let entry_path = path.join(&name);
                entries.push(FileEntry {
                    name,
                    path: entry_path,
                    size,
                    is_dir,
                    is_symlink: mode.contains('l') || mode.contains('L'),
                    modified,
                });
            }
        }
        Ok(entries)
    } else {
        anyhow::bail!("Failed to read directory as admin")
    }
}

#[cfg(not(target_os = "windows"))]
pub(super) fn read_directory_as_admin(path: &Path) -> Result<Vec<FileEntry>> {
    use std::io::Write;
    use std::process::Command;

    // The previous implementation built a Python `-c` command by string
    // interpolating the user-supplied path. Even though single quotes were
    // escaped, Python's single-quoted strings still treat a trailing `\`
    // followed by a newline as a line-continuation, so a path such as
    // `foo\<newline>print("pwned")` would break out of the string and
    // execute arbitrary Python as root (the whole point of using `sudo`
    // here). Other escapes (`\x..`, `\u..`, etc.) could similarly be
    // abused.
    //
    // The fix is to ship a static script and pass the path as a separate
    // argv element. Python receives the path on `sys.argv[1]` and never
    // re-parses it as source.
    let script_dir = std::env::temp_dir().join("pairee-admin");
    std::fs::create_dir_all(&script_dir)
        .map_err(|e| anyhow::anyhow!("Failed to create temp script directory: {}", e))?;
    let script_path = script_dir.join("ListDirectory.py");
    // Static body — no interpolation.
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
    if !script_path.exists() {
        std::fs::File::create(&script_path)
            .and_then(|mut f| f.write_all(script_body.as_bytes()))
            .map_err(|e| anyhow::anyhow!("Failed to write admin-listing script: {}", e))?;
    }

    let output = Command::new("sudo")
        .arg("python3")
        .arg("-B")
        .arg(&script_path)
        .arg(path)
        .output()?;
    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut entries = Vec::new();
        for line in stdout.lines() {
            let parts: Vec<&str> = line.split('|').collect();
            if parts.len() >= 5 {
                let name = parts[0].to_string();
                let size: u64 = parts[1].parse().unwrap_or(0);
                let is_dir = parts[2] == "1";
                let is_symlink = parts[3] == "1";
                let mtime: u64 = parts[4].parse().unwrap_or(0);
                let modified = if mtime > 0 {
                    Some(std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(mtime))
                } else {
                    None
                };
                let entry_path = path.join(&name);
                entries.push(FileEntry {
                    name,
                    path: entry_path,
                    size,
                    is_dir,
                    is_symlink,
                    modified,
                });
            }
        }
        return Ok(entries);
    }
    anyhow::bail!("Failed to read directory as admin via sudo")
}
