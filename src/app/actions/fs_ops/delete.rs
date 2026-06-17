use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType};
use crate::config::localization::t;

fn is_non_empty_dir(path: &std::path::Path) -> bool {
    if path.is_dir() {
        if let Ok(mut entries) = std::fs::read_dir(path) {
            entries.next().is_some()
        } else {
            false
        }
    } else {
        false
    }
}

pub fn handle(
    state: &mut AppState,
    context: &mut AppContext,
) -> bool {
    let targets = state.get_active_panel().get_targeted_paths();
    if !targets.is_empty() {
        let active_panel = state.get_active_panel();
        let is_remote = active_panel.ssh_conn.is_some();
        let show_prompt = context.config.settings.confirmations.confirm_delete
            || (context
                .config
                .settings
                .confirmations
                .confirm_delete_non_empty_folders
                && targets.iter().any(|p| {
                    if is_remote {
                        active_panel.entries.iter().any(|e| &e.path == p && e.is_dir)
                    } else {
                        is_non_empty_dir(p)
                    }
                }));

        if show_prompt {
            state.active_popup = Some(PopupType::ConfirmDelete {
                paths: targets,
                cursor_idx: 0,
            });
        } else {
            let active_panel = state.get_active_panel();
            if let Some(client) = &active_panel.ssh_conn {
                for path in &targets {
                    if let Err(e) = client.delete_recursive(path) {
                        state.active_popup = Some(PopupType::Error(format!(
                            "{} {}",
                            t("error_delete_failed"),
                            e
                        )));
                        return true;
                    }
                }
                state.get_active_panel_mut().clear_selection();
                state.refresh_both_panels(context.config.settings.show_hidden);
            } else {
                for path in &targets {
                    if let Err(e) = crate::fs::delete_sync(
                        path,
                        context.config.settings.delete_to_recycle_bin,
                        context.config.settings.req_admin_modification,
                    ) {
                        if !context.config.settings.req_admin_modification {
                            state.active_popup = Some(PopupType::ConfirmRetryAsAdmin {
                                paths: targets.clone(),
                                op_kind: crate::app::state::AdminOpKind::Delete,
                            });
                        } else {
                            state.active_popup = Some(PopupType::Error(format!(
                                "{} {}",
                                t("error_delete_failed"),
                                e
                            )));
                        }
                        return true;
                    }
                }
                if context.config.settings.req_admin_modification {
                    state.terminal_needs_clear = true;
                }
                state.get_active_panel_mut().clear_selection();
                state.refresh_both_panels(context.config.settings.show_hidden);
            }
        }
    }
    true
}
