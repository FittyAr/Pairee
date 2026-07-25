use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType};
use crate::config::localization::t;

pub fn handle(state: &mut AppState, _context: &mut AppContext) -> bool {
    let active = state.get_active_panel();
    if let Some(entry) = active.entries.get(active.cursor_index) {
        let original = entry.name.clone();
        state.active_popup = Some(PopupType::RenamePrompt {
            input: original.clone(),
            original,
            src_path: entry.path.clone(),
            parent_dir: active.current_path.clone(),
            cursor_idx: 0,
        });
        true
    } else {
        state.active_popup = Some(PopupType::Error(t("error_no_entry_rename")));
        true
    }
}

/// Perform the actual rename on Enter. Extracted so the input handler
/// can call it after the user confirms the new filename.
pub fn commit(
    state: &mut AppState,
    context: &mut AppContext,
    input: String,
    original: String,
    src_path: std::path::PathBuf,
    parent_dir: std::path::PathBuf,
) {
    let trimmed = input.trim().to_string();
    if trimmed.is_empty() || trimmed == original {
        state.active_popup = None;
        return;
    }
    let target = parent_dir.join(&trimmed);
    if target == src_path {
        state.active_popup = None;
        return;
    }
    match std::fs::rename(&src_path, &target) {
        Ok(_) => {
            if context.config.settings.req_admin_modification {
                state.terminal_needs_clear = true;
            }
            state.active_popup = None;
            state.refresh_both_panels(context.config.settings.show_hidden);
        }
        Err(e) => {
            if !context.config.settings.req_admin_modification {
                state.active_popup = Some(PopupType::ConfirmRetryAsAdmin {
                    paths: vec![src_path.clone()],
                    op_kind: crate::app::state::AdminOpKind::Rename {
                        src: src_path,
                        target,
                    },
                });
            } else {
                state.active_popup = Some(PopupType::Error(format!(
                    "{} {}",
                    t("error_rename_error"),
                    e
                )));
            }
        }
    }
}
