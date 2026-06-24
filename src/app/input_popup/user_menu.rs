use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType};
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent};

#[derive(Clone)]
pub struct UserMenuItem {
    pub key: String,
    pub label: String,
    pub command: Option<String>,
    pub action: Option<Action>,
}

pub fn get_user_menu_items() -> Vec<UserMenuItem> {
    let custom_cmds = crate::app::sys_helpers::load_user_menu_commands();
    let mut items = Vec::new();
    if !custom_cmds.is_empty() {
        for (k, v) in custom_cmds {
            items.push(UserMenuItem {
                key: k.clone(),
                label: v.clone(),
                command: Some(v),
                action: None,
            });
        }
    } else {
        // Defaults
        items.push(UserMenuItem {
            key: "1".to_string(),
            label: crate::config::localization::t("user_cmd_refresh"),
            command: None,
            action: Some(Action::Refresh),
        });
        items.push(UserMenuItem {
            key: "2".to_string(),
            label: crate::config::localization::t("user_cmd_toggle_hidden"),
            command: None,
            action: Some(Action::ToggleHidden),
        });
        items.push(UserMenuItem {
            key: "3".to_string(),
            label: crate::config::localization::t("user_cmd_swap"),
            command: None,
            action: Some(Action::SwapPanels),
        });
        items.push(UserMenuItem {
            key: "4".to_string(),
            label: crate::config::localization::t("user_cmd_help"),
            command: None,
            action: Some(Action::Help),
        });
        items.push(UserMenuItem {
            key: "5".to_string(),
            label: crate::config::localization::t("user_cmd_close"),
            command: None,
            action: None,
        }); // Closes menu
        items.push(UserMenuItem {
            key: "6".to_string(),
            label: crate::config::localization::t("user_cmd_download"),
            command: None,
            action: None,
        }); // Special download task
        items.push(UserMenuItem {
            key: "7".to_string(),
            label: crate::config::localization::t("user_cmd_screens"),
            command: None,
            action: Some(Action::ScreensList),
        });
        items.push(UserMenuItem {
            key: "8".to_string(),
            label: crate::config::localization::t("user_cmd_plugins"),
            command: None,
            action: Some(Action::PluginMenu),
        });
        items.push(UserMenuItem {
            key: "9".to_string(),
            label: crate::config::localization::t("user_cmd_git"),
            command: None,
            action: Some(Action::OpenGitPanel),
        });
        items.push(UserMenuItem {
            key: "0".to_string(),
            label: crate::config::localization::t("user_cmd_settings"),
            command: None,
            action: Some(Action::SystemSettings),
        });
    }
    // Always append Edit option at the end
    items.push(UserMenuItem {
        key: "E".to_string(),
        label: crate::config::localization::t("menu_edit_user_menu"),
        command: None,
        action: Some(Action::EditUserMenu),
    });
    items
}

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    if let Some(PopupType::UserMenu { mut cursor_idx }) = state.active_popup.clone() {
        let items = get_user_menu_items();

        match key.code {
            KeyCode::Esc => {
                state.active_popup = None;
                return Ok(None);
            }
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                if !items.is_empty() {
                    cursor_idx = if cursor_idx > 0 {
                        cursor_idx - 1
                    } else {
                        items.len() - 1
                    };
                    state.active_popup = Some(PopupType::UserMenu { cursor_idx });
                }
                return Ok(None);
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                if !items.is_empty() {
                    cursor_idx = if cursor_idx < items.len() - 1 {
                        cursor_idx + 1
                    } else {
                        0
                    };
                    state.active_popup = Some(PopupType::UserMenu { cursor_idx });
                }
                return Ok(None);
            }
            KeyCode::Enter => {
                if let Some(item) = items.get(cursor_idx) {
                    state.active_popup = None;
                    return execute_item(state, context, item);
                }
                state.active_popup = None;
                return Ok(None);
            }
            KeyCode::Char(c) => {
                let shortcut = c.to_string().to_uppercase();
                if let Some(item) = items.iter().find(|it| it.key.to_uppercase() == shortcut) {
                    state.active_popup = None;
                    return execute_item(state, context, item);
                }
                return Ok(None);
            }
            _ => {}
        }
        Err(())
    } else {
        Err(())
    }
}

fn execute_item(
    state: &mut AppState,
    context: &mut AppContext,
    item: &UserMenuItem,
) -> Result<Option<Action>, ()> {
    if let Some(act) = item.action {
        // Apply immediate settings checks for defaults
        if act == Action::Refresh {
            state.refresh_both_panels(context.config.settings.show_hidden);
            return Ok(None);
        } else if act == Action::ToggleHidden {
            context.config.settings.show_hidden = !context.config.settings.show_hidden;
            let _ = context.config.save();
            state.refresh_both_panels(context.config.settings.show_hidden);
            return Ok(None);
        } else if act == Action::SwapPanels {
            state.swap_panels();
            return Ok(None);
        }
        return Ok(Some(act));
    }

    if item.key == "5" {
        // Close menu
        return Ok(None);
    }

    if item.key == "6" {
        // Download external tools (7z)
        let (tx, rx) = tokio::sync::mpsc::channel(100);
        tokio::spawn(async move {
            let _ = tx
                .send(crate::fs::ProgressUpdate {
                    current_file: "Downloading 7z...".to_string(),
                    files_copied: 0,
                    total_files: 1,
                    bytes_copied: 0,
                    total_bytes: 1,
                    error: None,
                })
                .await;

            if let Err(e) = crate::fs::external_tools::ensure_external_tools().await {
                let _ = tx
                    .send(crate::fs::ProgressUpdate {
                        current_file: "Completed".to_string(),
                        files_copied: 0,
                        total_files: 1,
                        bytes_copied: 0,
                        total_bytes: 1,
                        error: Some(format!("Failed to download: {}", e)),
                    })
                    .await;
            } else {
                let _ = tx
                    .send(crate::fs::ProgressUpdate {
                        current_file: "Completed".to_string(),
                        files_copied: 1,
                        total_files: 1,
                        bytes_copied: 1,
                        total_bytes: 1,
                        error: None,
                    })
                    .await;
            }
        });

        state.progress_rx = Some(rx);
        state.active_popup = Some(PopupType::CopyProgress {
            is_move: false,
            current_file: "Initializing Download...".to_string(),
            files_copied: 0,
            total_files: 1,
            bytes_copied: 0,
            total_bytes: 1,
        });
        return Ok(None);
    }

    if let Some(cmd_template) = &item.command {
        let active_panel = state.get_active_panel();
        let highlighted = active_panel.entries.get(active_panel.cursor_index);
        let final_cmd = if let Some(e) = highlighted {
            cmd_template
                .replace("{f}", &e.name)
                .replace("{p}", &e.path.to_string_lossy())
        } else {
            cmd_template.clone()
        };
        state.pending_custom_command = Some(final_cmd);
        return Ok(None);
    }

    Ok(None)
}
