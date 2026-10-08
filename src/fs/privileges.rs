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
    use crossterm::cursor::Show;
    use crossterm::execute;
    use crossterm::terminal::{
        EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
    };
    use std::process::Stdio;

    let _ = disable_raw_mode();
    let _ = execute!(std::io::stdout(), LeaveAlternateScreen, Show);

    println!("\n{message}");

    let status = Command::new("sudo")
        .args(args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status();

    let _ = enable_raw_mode();
    let _ = execute!(std::io::stdout(), EnterAlternateScreen);
    status
}

pub fn run_in_elevated_helper(ops: Vec<FsOperation>) -> Result<()> {
    let temp_dir = std::env::temp_dir();
    // Use a UUID rather than the (guessable) process id so a local
    // attacker cannot pre-create the JSON file with their own operations
    // and have the helper execute them as root / as Administrator. Pairs
    // with the matching helper-side fix in `main.rs` which builds the
    // res-file path from the same operations envelope.
    let unique = uuid::Uuid::new_v4();
    let temp_file_path = temp_dir.join(format!("pairee_op_{}.json", unique));

    let json_content = serde_json::to_string(&ops)?;
    std::fs::write(&temp_file_path, json_content)?;

    let current_exe = std::env::current_exe()?;

    let run_res = run_helper_process(&current_exe, &temp_file_path);

    // Best-effort cleanup; if the helper crashed before deleting, the
    // unique name means it won't collide with a future run.
    let _ = std::fs::remove_file(&temp_file_path);

    run_res
}

#[cfg(target_os = "windows")]
fn run_helper_process(exe: &Path, temp_file: &Path) -> Result<()> {
    // Avoid string-interpolating the executable path and temp file path into
    // a PowerShell command. PowerShell re-parses the inner string for
    // metacharacters (`$`, `` ` ``, `"`, `;`), so a path containing any of
    // these could escape the quoted argument and execute arbitrary code.
    // Instead, we pass every argument to Start-Process as a separate
    // element of `-ArgumentList`, which PowerShell does not re-parse.
    let status = Command::new("powershell")
        .arg("-NoProfile")
        .arg("-Command")
        .arg("$exe = $args[0]; $tf = $args[1]; Start-Process -FilePath $exe -ArgumentList @('--elevated-helper', $tf) -Verb RunAs -WindowStyle Hidden -Wait")
        .arg(exe)
        .arg(temp_file)
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
