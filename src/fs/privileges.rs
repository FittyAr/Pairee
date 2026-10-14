use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum FsOperation {
    Delete { path: PathBuf },
    MkDir { path: PathBuf },
    Copy { src: PathBuf, dst: PathBuf },
    Move { src: PathBuf, dst: PathBuf },
    Chmod { path: PathBuf, mode: u32 },
}

#[cfg(target_os = "windows")]
pub fn is_elevated() -> bool {
    #[link(name = "shell32")]
    unsafe extern "system" {
        fn IsUserAnAdmin() -> i32;
    }
    unsafe { IsUserAnAdmin() != 0 }
}

#[cfg(not(target_os = "windows"))]
pub fn is_elevated() -> bool {
    unsafe extern "C" {
        fn geteuid() -> u32;
    }
    unsafe { geteuid() == 0 }
}

#[cfg(target_os = "windows")]
pub fn acquire_admin_privileges() -> Result<()> {
    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn acquire_admin_privileges() -> Result<()> {
    let status = run_sudo(
        "Requesting administrator privileges...",
        &[std::ffi::OsStr::new("-v")],
    );
    match status {
        Ok(s) if s.success() => Ok(()),
        _ => anyhow::bail!("Failed to acquire admin privileges via sudo"),
    }
}

/// Leaves the TUI, runs `sudo args` on the user's terminal (so it can ask
/// for the password) and restores the TUI afterwards.
#[cfg(not(target_os = "windows"))]
fn run_sudo(message: &str, args: &[&std::ffi::OsStr]) -> std::io::Result<std::process::ExitStatus> {
    use std::process::Stdio;

    let suspended = crate::terminal::suspend::suspend();

    println!("\n{message}");

    let status = Command::new("sudo")
        .args(args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status();

    if let Ok(suspended) = suspended {
        let _ = suspended.resume();
    }
    status
}

/// A new temporary folder only the user can enter (0700 on Unix; the
/// user's own temp folder on Windows), removed on drop. Elevated helpers
/// read and write their files there, out of other users' reach.
pub(crate) fn private_temp_dir(prefix: &str) -> std::io::Result<tempfile::TempDir> {
    let mut builder = tempfile::Builder::new();
    builder.prefix(prefix);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        builder.permissions(std::fs::Permissions::from_mode(0o700));
    }
    builder.tempdir()
}

pub fn run_in_elevated_helper(ops: Vec<FsOperation>) -> Result<()> {
    // The operations and the helper's result live in a private folder of
    // their own (0700 on Unix, random name): in the shared temp folder a
    // local attacker could swap the operations, or plant a link where the
    // helper — running as root — writes its result.
    let private = private_temp_dir("pairee-op-")?;
    let temp_file_path = private.path().join("ops.json");

    let json_content = serde_json::to_string(&ops)?;
    std::fs::write(&temp_file_path, json_content)?;

    let current_exe = std::env::current_exe()?;

    // The folder (with whatever the helper left in it) goes away on drop.
    run_helper_process(&current_exe, &temp_file_path)
}

/// A PowerShell single-quoted literal of `value`: nothing inside it is
/// evaluated. PowerShell also takes the typographic quotes U+2018 to
/// U+201B for single quotes, so they are doubled as well.
#[cfg(any(target_os = "windows", test))]
pub(crate) fn ps_literal(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('\'');
    for c in value.chars() {
        if matches!(c, '\'' | '\u{2018}' | '\u{2019}' | '\u{201A}' | '\u{201B}') {
            out.push(c);
        }
        out.push(c);
    }
    out.push('\'');
    out
}

/// A PowerShell process that runs `script` with `values` bound to `$v0`,
/// `$v1`, ... as literals. The code goes in `-EncodedCommand`: whatever
/// follows `-Command` on a command line is code, so paths passed there as
/// "arguments" were run (and `$args` stayed empty).
#[cfg(target_os = "windows")]
pub(crate) fn powershell(script: &str, values: &[&std::ffi::OsStr]) -> Result<Command> {
    use base64::Engine as _;
    let mut code = String::new();
    for (i, value) in values.iter().enumerate() {
        let value = value
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Path is not valid Unicode: {value:?}"))?;
        code.push_str(&format!("$v{i} = {}; ", ps_literal(value)));
    }
    code.push_str(script);
    let utf16: Vec<u8> = code.encode_utf16().flat_map(u16::to_le_bytes).collect();
    let encoded = base64::engine::general_purpose::STANDARD.encode(utf16);
    let mut command = Command::new("powershell");
    command
        .arg("-NoProfile")
        .arg("-NonInteractive")
        .arg("-EncodedCommand")
        .arg(encoded);
    Ok(command)
}

#[cfg(target_os = "windows")]
fn run_helper_process(exe: &Path, temp_file: &Path) -> Result<()> {
    // `Start-Process` joins the argument list with spaces: the path is
    // quoted for the helper's own command line.
    let status = powershell(
        "Start-Process -FilePath $v0 -ArgumentList @('--elevated-helper', ('\"' + $v1 + '\"')) -Verb RunAs -WindowStyle Hidden -Wait",
        &[exe.as_os_str(), temp_file.as_os_str()],
    )?
    .status()
    .context("Failed to run elevated helper via PowerShell")?;

    if !status.success() {
        anyhow::bail!("Failed to acquire Administrator privileges (UAC prompt declined or failed)")
    }
    read_helper_result(temp_file)
}

#[cfg(not(target_os = "windows"))]
fn run_helper_process(exe: &Path, temp_file: &Path) -> Result<()> {
    let status = run_sudo(
        "Requesting administrator privileges to complete operation...",
        &[
            exe.as_os_str(),
            std::ffi::OsStr::new("--elevated-helper"),
            temp_file.as_os_str(),
        ],
    )
    .context("Failed to run elevated helper via sudo")?;

    if !status.success() {
        anyhow::bail!("Failed to run elevated operation via sudo")
    }
    read_helper_result(temp_file)
}

/// Reads (and removes) the `.res` file the elevated helper writes next to
/// its operations file: `OK` on success, otherwise the error message.
fn read_helper_result(temp_file: &Path) -> Result<()> {
    let res_file = temp_file.with_extension("res");
    if !res_file.exists() {
        anyhow::bail!("Elevated helper terminated without writing result status")
    }
    let res_content = std::fs::read_to_string(&res_file)?;
    let _ = std::fs::remove_file(&res_file);
    if res_content == "OK" {
        Ok(())
    } else {
        anyhow::bail!("Elevated helper error: {}", res_content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn powershell_literals_never_evaluate() {
        assert_eq!(ps_literal("C:\\a b"), "'C:\\a b'");
        assert_eq!(ps_literal("it's"), "'it''s'");
        assert_eq!(ps_literal("x\u{2019}y"), "'x\u{2019}\u{2019}y'");
        assert_eq!(ps_literal("$(calc)"), "'$(calc)'");
    }

    /// Values reach the script as data, whatever they contain.
    #[cfg(target_os = "windows")]
    #[test]
    fn powershell_values_arrive_unchanged() {
        let value = std::ffi::OsStr::new("a'b $(exit 7) \u{2018}c");
        let status = powershell(
            "if ($v0 -ceq ('a''b $(exit 7) ' + [char]0x2018 + 'c')) { exit 0 } else { exit 3 }",
            &[value],
        )
        .unwrap()
        .status()
        .unwrap();
        assert_eq!(status.code(), Some(0));
    }

    #[test]
    fn test_operation_serialization() {
        let ops = vec![
            FsOperation::MkDir {
                path: PathBuf::from("test/dir"),
            },
            FsOperation::Delete {
                path: PathBuf::from("test/file"),
            },
            FsOperation::Copy {
                src: PathBuf::from("test/src"),
                dst: PathBuf::from("test/dst"),
            },
        ];
        let json = serde_json::to_string(&ops).expect("serialize");
        let parsed: Vec<FsOperation> = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed.len(), 3);
        match &parsed[0] {
            FsOperation::MkDir { path } => assert_eq!(path, Path::new("test/dir")),
            _ => panic!("Expected MkDir"),
        }
    }
}
