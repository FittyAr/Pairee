use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

#[cfg(target_os = "windows")]
fn run_as_admin_mkdir(path: &Path) -> Result<()> {
    use std::process::Command;
    let path_str = path.to_string_lossy().replace('"', "\\\"");
    let ps_arg = format!(
        "Start-Process powershell -ArgumentList '-NoProfile -Command New-Item -ItemType Directory -Path \\\"{}\\\" -Force' -Verb RunAs -WindowStyle Hidden",
        path_str
    );
    let status = Command::new("powershell")
        .args(&["-NoProfile", "-Command", &ps_arg])
        .status()?;
    if status.success() {
        Ok(())
    } else {
        anyhow::bail!("Failed to create folder as administrator")
    }
}

#[cfg(not(target_os = "windows"))]
fn run_as_admin_mkdir(path: &Path) -> Result<()> {
    use crossterm::cursor::Show;
    use crossterm::execute;
    use crossterm::terminal::{
        EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
    };
    use std::process::{Command, Stdio};

    let _ = disable_raw_mode();
    let _ = execute!(std::io::stdout(), LeaveAlternateScreen, Show);

    println!(
        "\nRequesting administrator privileges to create folder: {}",
        path.display()
    );

    let status = Command::new("sudo")
        .arg("mkdir")
        .arg("-p")
        .arg(path)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status();

    let _ = enable_raw_mode();
    let _ = execute!(std::io::stdout(), EnterAlternateScreen);

    match status {
        Ok(s) if s.success() => Ok(()),
        _ => anyhow::bail!("Failed to create folder as administrator via sudo"),
    }
}

/// Creates a new directory at the specified path.
pub fn create_directory(path: &Path, req_admin: bool) -> Result<()> {
    let res = fs::create_dir(path).context("Failed to create directory");
    if res.is_err() && req_admin {
        run_as_admin_mkdir(path)
    } else {
        res
    }
}
