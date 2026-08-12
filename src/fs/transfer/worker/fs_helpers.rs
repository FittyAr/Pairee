#[cfg(target_os = "windows")]
pub(super) fn send_to_recycle_bin_helper(path: &std::path::Path) -> anyhow::Result<()> {
    use std::io::Write;
    use std::process::Command;

    // The previous implementation built a PowerShell command by string-
    // concatenation of the user-supplied file path. Any path containing a
    // PowerShell metacharacter (`'`, `$`, `` ` ``, `"`, `;`, etc.) could
    // break out of the quoted argument and execute arbitrary PowerShell.
    //
    // The fix is to ship a static script and pass the path as a parameter
    // (which PowerShell never re-parses for metacharacters). We write the
    // script to a uniquely-named temp file and pass the path as the
    // `-Path` argument so the file name is never concatenated into source.
    let script_dir = std::env::temp_dir().join("pairee-recycle");
    std::fs::create_dir_all(&script_dir)
        .map_err(|e| anyhow::anyhow!("Failed to create temp script directory: {}", e))?;
    let script_path = script_dir.join("SendToRecycleBin.ps1");
    // The script body is fully static; the path is delivered as a parameter.
    let script_body = r#"param(
    [Parameter(Mandatory = $true, Position = 0)]
    [string] $Path,
    [Parameter(Mandatory = $true)]
    [bool] $IsDirectory
)

$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName Microsoft.VisualBasic

if ($IsDirectory) {
    [Microsoft.VisualBasic.FileIO.FileSystem]::DeleteDirectory(
        $Path,
        'OnlyErrorDialogs',
        'SendToRecycleBin'
    )
} else {
    [Microsoft.VisualBasic.FileIO.FileSystem]::DeleteFile(
        $Path,
        'OnlyErrorDialogs',
        'SendToRecycleBin'
    )
}
"#;
    if !script_path.exists() {
        let mut f = std::fs::File::create(&script_path)
            .map_err(|e| anyhow::anyhow!("Failed to write recycle helper script: {}", e))?;
        f.write_all(script_body.as_bytes())?;
    }

    let is_dir = path.is_dir();
    let output = Command::new("powershell")
        .arg("-NoProfile")
        .arg("-ExecutionPolicy")
        .arg("Bypass")
        .arg("-File")
        .arg(&script_path)
        .arg("-Path")
        .arg(path)
        .arg("-IsDirectory")
        .arg(if is_dir { "true" } else { "false" })
        .output();

    let output = match output {
        Ok(o) => o,
        Err(e) => anyhow::bail!("Failed to execute PowerShell trash command: {}", e),
    };
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("PowerShell Recycle Bin error: {}", err);
    }
    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub(super) fn send_to_recycle_bin_helper(path: &std::path::Path) -> anyhow::Result<()> {
    use std::process::Command;
    let status = Command::new("gio")
        .arg("trash")
        .arg("--")
        .arg(path)
        .status();
    if let Ok(s) = status {
        if s.success() {
            return Ok(());
        }
    }
    let status = Command::new("trash-put").arg("--").arg(path).status();
    if let Ok(s) = status {
        if s.success() {
            return Ok(());
        }
    }
    // Fallback to standard delete if trash command fails
    if path.is_dir() {
        std::fs::remove_dir_all(path)
            .map_err(|e| anyhow::anyhow!("Failed to delete dir recursively: {}", e))
    } else {
        std::fs::remove_file(path).map_err(|e| anyhow::anyhow!("Failed to delete file: {}", e))
    }
}

pub(super) fn make_writable_helper(path: &std::path::Path) -> std::io::Result<()> {
    let metadata = path.symlink_metadata()?;
    if metadata.file_type().is_symlink() {
        return Ok(());
    }
    let mut perms = metadata.permissions();
    #[cfg(not(target_os = "windows"))]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = perms.mode();
        let is_dir = metadata.is_dir();
        let new_mode = if is_dir { mode | 0o700 } else { mode | 0o600 };
        perms.set_mode(new_mode);
    }
    #[cfg(target_os = "windows")]
    {
        // Windows: clear readonly bit before delete
        #[allow(clippy::permissions_set_readonly_false)]
        perms.set_readonly(false);
    }
    std::fs::set_permissions(path, perms)
}
