use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType};
use crate::config::localization::t;
use crate::fs::remote::RemoteConn;
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

/// Runs the user's command line `command_str` in the active panel's folder:
/// on the local machine, or on the server of an SSH panel. A panel that
/// cannot run commands (FTP, WebDAV, S3, inside an archive) gets an error
/// dialog instead of a local command run in a folder that does not exist.
pub fn run_in_active_panel(
    state: &mut AppState,
    command_str: &str,
    context: &AppContext,
    terminal_backend: &mut TerminalBackend,
) -> Result<()> {
    let panel = state.get_active_panel();
    let (dir, source) = (panel.current_path.clone(), panel.source.clone());
    let caps = source.capabilities();
    if caps.local_tools {
        return execute_shell_command(command_str, &dir, context, terminal_backend);
    }
    match source.remote() {
        Some(conn) if caps.run_commands => {
            execute_remote_command(command_str, &dir, conn, terminal_backend)
        }
        _ => {
            state
                .dialogs
                .replace(PopupType::error(t("shell_remote_unsupported")));
            Ok(())
        }
    }
}

/// Suspends the TUI **in-place**, runs a shell command natively, then
/// restores it.
fn execute_shell_command(
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
        println!("\n{}\n", t("shell_running").replacen("{}", command_str, 1));
        let status = crate::shell::shell_command(command_str)
            .current_dir(current_dir)
            .stdin(std::process::Stdio::inherit())
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .status();
        report_outcome(status.map(|s| s.code()));
    })?;
    Ok(())
}

/// Suspends the TUI and runs `command_str` in `dir` on the server of
/// `conn`, its output shown as it arrives (no input: the command runs
/// without a terminal of its own).
fn execute_remote_command(
    command_str: &str,
    dir: &Path,
    conn: &RemoteConn,
    terminal_backend: &mut TerminalBackend,
) -> Result<()> {
    with_suspended_tui(terminal_backend, || {
        println!(
            "\n{}\n",
            t("shell_remote_running")
                .replacen("{}", &conn.user_at_host(), 1)
                .replacen("{}", command_str, 1)
        );
        let status = conn.run_command(dir, command_str, &mut std::io::stdout());
        report_outcome(status.map(Some));
    })?;
    Ok(())
}

/// Prints why a command could not start, or its exit status when it failed
/// (a success prints nothing).
fn report_outcome(status: std::io::Result<Option<i32>>) {
    match status {
        Ok(Some(0)) => {}
        Ok(Some(code)) => println!(
            "\n{}",
            t("shell_exit_status").replacen("{}", &code.to_string(), 1)
        ),
        // Ended by a signal (Unix): the shell already said so.
        Ok(None) => {}
        Err(e) => eprintln!("\n{e}"),
    }
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
