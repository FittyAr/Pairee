use crate::app::context::AppContext;
use crate::app::form::FormKey;
use crate::app::state::popup::forms::MKDIR_FORM;
use crate::app::state::{AppState, PopupType};
use crate::config::localization::t;
use crate::keybindings::Action;
use crossterm::event::KeyEvent;

const ROW_MULTIPLE: usize = 1;
const BUTTON_CANCEL: usize = 3;

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    let Some(PopupType::MkDirPrompt {
        input,
        cursor_idx,
        process_multiple,
    }) = state.dialogs.top_mut()
    else {
        return Err(());
    };
    match MKDIR_FORM.handle(cursor_idx, Some(input), &key) {
        FormKey::Toggle(ROW_MULTIPLE) => *process_multiple = !*process_multiple,
        FormKey::Activate(BUTTON_CANCEL) | FormKey::Cancel => state.dialogs.clear(),
        FormKey::Activate(_) => {
            let name = input.text().to_string();
            create(state, context, &name);
        }
        FormKey::Toggle(_) | FormKey::Handled | FormKey::Other => {}
    }
    Ok(None)
}

/// Creates `name` in the active panel (offering an elevated retry on failure).
fn create(state: &mut AppState, context: &AppContext, name: &str) {
    if name.is_empty() {
        state.dialogs.clear();
        return;
    }
    let settings = &context.config.settings;
    let path = state.get_active_panel().current_path.join(name);
    match crate::fs::create_directory(&path, settings.req_admin_modification) {
        Err(_) if !settings.req_admin_modification => {
            state.dialogs.replace(PopupType::ConfirmRetryAsAdmin {
                paths: vec![path],
                op_kind: crate::app::state::AdminOpKind::MkDir,
            });
        }
        Err(e) => {
            state
                .dialogs
                .replace(PopupType::Error(format!("{} {}", t("error_dir_error"), e)))
        }
        Ok(()) => {
            if settings.req_admin_modification {
                state.terminal_needs_clear = true;
            }
            state.dialogs.clear();
            state.refresh_both_panels(settings.show_hidden);
        }
    }
}
