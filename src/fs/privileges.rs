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
    use crossterm::cursor::Show;
    use crossterm::execute;
    use crossterm::terminal::{
        EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
    };
    use std::process::Stdio;

    let _ = disable_raw_mode();
    let _ = execute!(std::io::stdout(), LeaveAlternateScreen, Show);

    println!("\nRequesting administrator privileges...");

    let status = Command::new("sudo")
        .arg("-v")
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status();

    let _ = enable_raw_mode();
    let _ = execute!(std::io::stdout(), EnterAlternateScreen);

    match status {
        Ok(s) if s.success() => Ok(()),
        _ => anyhow::bail!("Failed to acquire admin privileges via sudo"),
    }
}

pub fn run_in_elevated_helper(ops: Vec<FsOperation>) -> Result<()> {
    let temp_dir = std::env::temp_dir();
    let temp_file_path = temp_dir.join(format!("pairee_op_{}.json", std::process::id()));

    let json_content = serde_json::to_string(&ops)?;
    std::fs::write(&temp_file_path, json_content)?;

    let current_exe = std::env::current_exe()?;

    let run_res = run_helper_process(&current_exe, &temp_file_path);

    let _ = std::fs::remove_file(&temp_file_path);

    run_res
}

#[cfg(target_os = "windows")]
fn run_helper_process(exe: &Path, temp_file: &Path) -> Result<()> {
    let exe_str = exe.to_string_lossy().replace('"', "\\\"");
    let temp_str = temp_file.to_string_lossy().replace('"', "\\\"");

    let ps_arg = format!(
        "Start-Process -FilePath \"{}\" -ArgumentList \"--elevated-helper \\\"{}\\\"\" -Verb RunAs -WindowStyle Hidden -Wait",
        exe_str, temp_str
    );

    let status = Command::new("powershell")
        .args(&["-NoProfile", "-Command", &ps_arg])
        .status()
        .context("Failed to run elevated helper via PowerShell")?;

    if status.success() {
        let res_file = temp_file.with_extension("res");
        if res_file.exists() {
            let res_content = std::fs::read_to_string(&res_file)?;
            let _ = std::fs::remove_file(&res_file);
            if res_content == "OK" {
                Ok(())
            } else {
                anyhow::bail!("Elevated helper error: {}", res_content)
            }
        } else {
            anyhow::bail!("Elevated helper terminated without writing result status")
        }
    } else {
        anyhow::bail!("Failed to acquire Administrator privileges (UAC prompt declined or failed)")
    }
}

#[cfg(not(target_os = "windows"))]
fn run_helper_process(exe: &Path, temp_file: &Path) -> Result<()> {
    use crossterm::cursor::Show;
    use crossterm::execute;
    use crossterm::terminal::{
        EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
    };
    use std::process::Stdio;

    let _ = disable_raw_mode();
    let _ = execute!(std::io::stdout(), LeaveAlternateScreen, Show);

    println!("\nRequesting administrator privileges to complete operation...");

    let status = Command::new("sudo")
        .arg(exe)
        .arg("--elevated-helper")
        .arg(temp_file)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .context("Failed to run elevated helper via sudo")?;

    let _ = enable_raw_mode();
    let _ = execute!(std::io::stdout(), EnterAlternateScreen);

    if status.success() {
        let res_file = temp_file.with_extension("res");
        if res_file.exists() {
            let res_content = std::fs::read_to_string(&res_file)?;
            let _ = std::fs::remove_file(&res_file);
            if res_content == "OK" {
                Ok(())
            } else {
                anyhow::bail!("Elevated helper error: {}", res_content)
            }
        } else {
            anyhow::bail!("Elevated helper terminated without writing result status")
        }
    } else {
        anyhow::bail!("Failed to run elevated operation via sudo")
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
