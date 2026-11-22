use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType, Screen, TerminalState};
use crate::config::localization::t;
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

/// Runs the user's command line `command_str` in the active panel's folder,
/// on a terminal screen inside Pairee's layout: on the local machine, or on
/// a terminal of the server of an SSH panel. Interactive programs work
/// there (an emulated terminal), the panels come back with the screens
/// keys while it runs, and Enter closes it once it ended. A panel that
/// cannot run commands (FTP, WebDAV, S3, inside an archive) gets an error
/// dialog instead of a local command run in a folder that does not exist.
pub fn run_in_active_panel(state: &mut AppState, command_str: &str, context: &AppContext) {
    let command = command_str.trim().to_string();
    if command.is_empty() {
        return;
    }
    let limit = state.screens.terminal_lines;
    let panel = state.get_active_panel();
    let (dir, source) = (panel.current_path.clone(), panel.source.clone());
    let caps = source.capabilities();
    let ts = if caps.local_tools {
        if context.config.settings.automatic_update_env_variables {
            crate::app::sys_helpers::refresh_env_vars();
        }
        TerminalState::spawn(command, &dir, limit)
    } else if let Some(conn) = source.remote().filter(|_| caps.run_commands) {
        TerminalState::spawn_remote(command, conn.clone(), dir, limit)
    } else {
        state
            .dialogs
            .replace(PopupType::error(t("shell_remote_unsupported")));
        return;
    };
    state.push_screen(Screen::Terminal(ts));
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
