use crate::app::state::{AppState, PopupType};
use crate::app::sys_helpers::clipboard;
use crate::config::localization::t;

pub fn handle(state: &mut AppState) -> bool {
    let text = {
        let panel = state.get_active_panel();
        clipboard::format_paths(&clipboard::paths_to_copy(panel))
    };
    match clipboard::set_text(&text) {
        Ok(()) => {
            state
                .dialogs
                .replace(PopupType::Info(t("clipboard_copied").replace("{}", &text)));
        }
        Err(e) => {
            state
                .dialogs
                .replace(PopupType::Error(t("clipboard_failed").replace("{}", &e)));
        }
    }
    true
}
