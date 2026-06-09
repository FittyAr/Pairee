use crate::app::context::AppContext;
use crate::terminal::TerminalBackend;
use anyhow::Result;
use crossterm::{
    cursor::Show,
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use std::path::Path;

/// Suspends raw mode **in-place**, runs a shell command natively, then re-enables raw mode.
/// Does NOT drop/recreate TerminalBackend to avoid double-restore.
pub fn execute_shell_command(
    command_str: &str,
    current_dir: &Path,
    context: &AppContext,
    terminal_backend: &mut TerminalBackend,
) -> Result<()> {
    if context.config.settings.automatic_update_env_variables {
        crate::app::sys_helpers::refresh_env_vars();
    }

    // Suspend TUI: leave alternate screen, disable raw mode
    terminal_backend.terminal.flush()?;
    disable_raw_mode()?;
    execute!(std::io::stdout(), LeaveAlternateScreen, Show)?;

    println!("\nNCRust shell execution: {}\n", command_str);

    let interactive = [
        "vi", "vim", "nvim", "nano", "pico", "emacs", "htop", "top", "mc", "less", "more", "ssh", "gdb",
        "python", "node", "mysql", "psql", "sqlite3", "bash", "sh", "zsh", "fish", "tmux", "screen",
        "ftp", "sftp", "telnet", "w3m", "lynx", "su", "sudo", "login",
    ];
    let cmd_name = command_str.split_whitespace().next().unwrap_or("").to_lowercase();
    let is_interactive = interactive.contains(&cmd_name.as_str());

    let final_command = if !is_interactive && !cfg!(target_os = "windows") {
        if command_str.contains("| less") || command_str.contains("| more") {
            command_str.to_string()
        } else {
            format!("({}) 2>&1 | less -F -R -X", command_str)
        }
    } else {
        command_str.to_string()
    };

    let mut shell = if cfg!(target_os = "windows") {
        std::process::Command::new("cmd")
            .arg("/c")
            .arg(&final_command)
            .current_dir(current_dir)
            .stdin(std::process::Stdio::inherit())
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .spawn()?
    } else {
        std::process::Command::new("sh")
            .arg("-c")
            .arg(&final_command)
            .current_dir(current_dir)
            .stdin(std::process::Stdio::inherit())
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .spawn()?
    };

    let _ = shell.wait();

    println!("\n[Press Enter to return to NCRust]");
    let mut buffer = String::new();
    let _ = std::io::stdin().read_line(&mut buffer);

    // Resume TUI: re-enable raw mode and re-enter alternate screen
    enable_raw_mode()?;
    execute!(std::io::stdout(), EnterAlternateScreen)?;
    terminal_backend.terminal.clear()?;
    Ok(())
}

// Suspends TUI and launches an external editor or viewer command (reserved for custom user command association bindings).
pub fn execute_external_command(
    _target_path: &Path,
    utility_command: &str,
    context: &AppContext,
    terminal_backend: &mut TerminalBackend,
) -> Result<()> {
    if context.config.settings.automatic_update_env_variables {
        crate::app::sys_helpers::refresh_env_vars();
    }

    // Suspend TUI
    terminal_backend.terminal.flush()?;
    disable_raw_mode()?;
    execute!(std::io::stdout(), LeaveAlternateScreen, Show)?;

    let shell = if cfg!(target_os = "windows") {
        "cmd"
    } else {
        "sh"
    };
    let flag = if cfg!(target_os = "windows") {
        "/c"
    } else {
        "-c"
    };
    let mut child = std::process::Command::new(shell)
        .arg(flag)
        .arg(utility_command)
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .spawn()?;

    let _ = child.wait();

    println!("\n[Press Enter to return to NCRust]");
    let mut buffer = String::new();
    let _ = std::io::stdin().read_line(&mut buffer);

    // Resume TUI
    enable_raw_mode()?;
    execute!(std::io::stdout(), EnterAlternateScreen)?;
    terminal_backend.terminal.clear()?;
    Ok(())
}
