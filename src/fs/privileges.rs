#[cfg(not(target_os = "windows"))]
pub fn acquire_admin_privileges() -> anyhow::Result<()> {
    use crossterm::cursor::Show;
    use crossterm::execute;
    use crossterm::terminal::{
        EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
    };
    use std::process::{Command, Stdio};

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
