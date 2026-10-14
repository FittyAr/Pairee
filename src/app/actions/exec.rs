use crate::app::context::AppContext;
use crate::terminal::TerminalBackend;
use anyhow::Result;
use std::path::Path;

/// Hands the terminal to `run`, waits for Enter, then gives it back to the
/// TUI. Does NOT drop/recreate TerminalBackend to avoid double-restore.
fn with_suspended_tui<T>(
    terminal_backend: &mut TerminalBackend,
    run: impl FnOnce() -> T,
) -> Result<T> {
    terminal_backend.terminal.flush()?;
    let suspended = crate::terminal::suspend::suspend()?;
    let result = run();
    println!("\n{}", crate::config::localization::t("shell_press_enter"));
    let mut buffer = String::new();
    let _ = std::io::stdin().read_line(&mut buffer);
    suspended.resume()?;
    terminal_backend.terminal.clear()?;
    Ok(result)
}

/// Suspends the TUI **in-place**, runs a shell command natively, then
/// restores it.
pub fn execute_shell_command(
    command_str: &str,
    current_dir: &Path,
    context: &AppContext,
    terminal_backend: &mut TerminalBackend,
) -> Result<()> {
    if context.config.settings.automatic_update_env_variables {
        crate::app::sys_helpers::refresh_env_vars();
    }

    // The command runs as typed, on the real terminal: interactive programs
    // (python, `git commit`, `docker run -it`...) work, and its output stays
    // on screen until Enter (no pager wrapped around it).
    with_suspended_tui(terminal_backend, || {
        println!(
            "\n{}\n",
            crate::config::localization::t("shell_running").replacen("{}", command_str, 1)
        );
        crate::shell::shell_command(command_str)
            .current_dir(current_dir)
            .stdin(std::process::Stdio::inherit())
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .spawn()
            .map(|mut shell| {
                let _ = shell.wait();
            })
    })??;
    Ok(())
}

// Suspends TUI and launches an external program with a fully-resolved argv.
//
// This intentionally does NOT route through a shell, so a file path that
// contains shell metacharacters cannot trigger command injection. Use this
// for file-association handlers where the file path is derived from a
// (potentially attacker-controlled) filename.
pub fn execute_external_program(
    _target_path: &Path,
    program: &str,
    args: &[String],
    context: &AppContext,
    terminal_backend: &mut TerminalBackend,
) -> Result<()> {
    if context.config.settings.automatic_update_env_variables {
        crate::app::sys_helpers::refresh_env_vars();
    }

    if program.is_empty() {
        anyhow::bail!("Cannot execute empty program");
    }

    let status = with_suspended_tui(terminal_backend, || {
        std::process::Command::new(program)
            .args(args)
            .stdin(std::process::Stdio::inherit())
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .status()
    })?;

    match status {
        Ok(s) if s.success() => Ok(()),
        Ok(s) => anyhow::bail!("Program exited with status {}", s),
        Err(e) => Err(e.into()),
    }
}
