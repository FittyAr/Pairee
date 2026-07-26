use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType};
use crate::config::localization::t;
use crate::keybindings::Action;
use crate::update::{UpdateStatus, detect::detect_install_method, installer};
use crossterm::event::{KeyCode, KeyEvent};
use tokio::sync::mpsc;

/// Handle keyboard input for the UpdateAvailable popup.
/// Returns Ok(None) if consumed, Ok(Some(action)) to bubble up, Err(()) to ignore.
pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    let (info_clone, cursor_idx) = match &state.active_popup {
        Some(PopupType::UpdateAvailable {
            info,
            cursor_idx,
            install_progress,
            ..
        }) => {
            // Block navigation while installing, but allow Esc or 'q' to close the popup
            if install_progress.is_some() {
                if key.code == KeyCode::Esc || key.code == KeyCode::Char('q') {
                    state.active_popup = None;
                }
                return Ok(None);
            }
            (info.clone(), *cursor_idx)
        }
        _ => return Err(()),
    };

    match key.code {
        // Scroll release notes
        KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
            if let Some(PopupType::UpdateAvailable { scroll_y, .. }) = &mut state.active_popup {
                *scroll_y = scroll_y.saturating_sub(1);
            }
            Ok(None)
        }
        KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
            if let Some(PopupType::UpdateAvailable { scroll_y, .. }) = &mut state.active_popup {
                *scroll_y = scroll_y.saturating_add(1);
            }
            Ok(None)
        }
        KeyCode::PageUp => {
            if let Some(PopupType::UpdateAvailable { scroll_y, .. }) = &mut state.active_popup {
                *scroll_y = scroll_y.saturating_sub(5);
            }
            Ok(None)
        }
        KeyCode::PageDown => {
            if let Some(PopupType::UpdateAvailable { scroll_y, .. }) = &mut state.active_popup {
                *scroll_y = scroll_y.saturating_add(5);
            }
            Ok(None)
        }

        // Navigate buttons
        KeyCode::Left | KeyCode::BackTab | KeyCode::Char('h') => {
            if let Some(PopupType::UpdateAvailable { cursor_idx, .. }) = &mut state.active_popup {
                *cursor_idx = cursor_idx.saturating_sub(1);
            }
            Ok(None)
        }
        KeyCode::Right | KeyCode::Tab | KeyCode::Char('l') => {
            if let Some(PopupType::UpdateAvailable { cursor_idx, .. }) = &mut state.active_popup {
                *cursor_idx = (*cursor_idx + 1).min(2);
            }
            Ok(None)
        }

        KeyCode::Enter => {
            match cursor_idx {
                // 0 = "Update now" / "Copy command"
                0 => {
                    let method = detect_install_method();
                    if method.is_managed() {
                        // Copy command to clipboard (best-effort)
                        if let Some(cmd) = method.managed_upgrade_command() {
                            copy_to_clipboard(&cmd);
                            state.active_popup = None;
                            state.active_popup =
                                Some(PopupType::Info(t("update_cmd_copied").replace("{}", &cmd)));
                        }
                    } else {
                        // Start the actual self-update
                        let (progress_tx, progress_rx) = mpsc::channel::<f32>(64);
                        state.update_progress_rx = Some(progress_rx);
                        state.update_status = UpdateStatus::Downloading(0.0);

                        // Set the progress marker in popup
                        if let Some(PopupType::UpdateAvailable {
                            install_progress, ..
                        }) = &mut state.active_popup
                        {
                            *install_progress = Some(0.0);
                        }

                        let (install_tx, install_rx) = tokio::sync::oneshot::channel();
                        state.update_install_rx = Some(install_rx);

                        // Spawn background task
                        let info_clone2 = info_clone.clone();
                        tokio::spawn(async move {
                            let result =
                                installer::perform_update(&info_clone2, &method, progress_tx).await;
                            let _ = install_tx.send(result.map_err(|e| e.to_string()));
                        });
                    }
                }
                // 1 = "Remind me later" — close popup, will show again next session
                1 => {
                    state.active_popup = None;
                }
                // 2 = "Ignore this version" — save dismissed tag to settings
                2 => {
                    context.config.settings.dismissed_update_version = Some(info_clone.tag.clone());
                    let _ = context.config.save();
                    state.active_popup = None;
                    state.update_available = None;
                }
                _ => {}
            }
            Ok(None)
        }

        KeyCode::Esc | KeyCode::Char('q') => {
            state.active_popup = None;
            Ok(None)
        }

        _ => Ok(None),
    }
}

fn copy_to_clipboard(text: &str) {
    // Try wl-copy / xclip / xsel on Linux; clip.exe on Windows.
    //
    // We used to check only that `spawn()` succeeded and then return
    // immediately, which meant a tool that launched but failed at runtime
    // (e.g. `wl-copy` with no Wayland session) would silently leave the
    // clipboard untouched. The UI would then show "copied" while the
    // user had nothing in their clipboard. Now we wait for the child and
    // require a successful exit status before moving on; on failure we
    // try the next tool in the list.
    #[cfg(not(target_os = "windows"))]
    {
        // Each entry: (program, args-before-stdin, use-stdin)
        //
        // `wl-copy` accepts the text as a positional argument and we let
        // it — passing the same text on stdin is undefined behaviour for
        // some implementations. `xclip` and `xsel` need the text on stdin
        // because their arguments only select the target clipboard.
        let candidates: &[(&str, &[&str], bool)] = &[
            ("wl-copy", &[], false),
            ("xclip", &["-selection", "clipboard"], true),
            ("xsel", &["--clipboard", "--input"], true),
        ];

        for (cmd, args, use_stdin) in candidates {
            match try_copy_with(cmd, args, text, *use_stdin) {
                Ok(()) => return,
                Err(e) => log::debug!("clipboard tool `{}` failed: {}", cmd, e),
            }
        }
    }
    #[cfg(target_os = "windows")]
    {
        let _ = try_copy_with("clip", &[], text, true);
    }
}

/// Spawn `cmd` with `args`, optionally feed `text` on its stdin, and return
/// `Ok(())` only if the child exited with status 0. Any spawn failure,
/// stdin-write failure, or non-zero exit is reported as an error so the
/// caller can try the next clipboard tool.
fn try_copy_with(cmd: &str, args: &[&str], text: &str, feed_stdin: bool) -> std::io::Result<()> {
    use std::io::Write as _;
    use std::process::{Command, Stdio};

    let mut command = Command::new(cmd);
    command.args(args);
    if feed_stdin {
        command.stdin(Stdio::piped());
    }
    let mut child = match command.spawn() {
        Ok(c) => c,
        Err(e) => {
            return Err(std::io::Error::new(
                e.kind(),
                format!("failed to spawn `{}`: {}", cmd, e),
            ));
        }
    };

    if feed_stdin {
        if let Some(stdin) = child.stdin.as_mut() {
            // We deliberately ignore short writes here — the user-visible
            // signal is the child's exit status, which we check below.
            let _ = stdin.write_all(text.as_bytes());
        } else {
            // We asked for piped stdin but the child didn't get one;
            // that's unexpected but not fatal, just continue.
            log::debug!("`{}` had no stdin pipe even though we asked for one", cmd);
        }
        // Drop stdin to close the pipe so `cmd` sees EOF and can exit.
        drop(child.stdin.take());
    } else {
        // `wl-copy` got the text as an arg; no stdin involved.
    }

    let status = child.wait()?;
    if status.success() {
        Ok(())
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("`{}` exited with {}", cmd, status),
        ))
    }
}
