use crate::config::settings::Settings;

pub fn handle_row(
    cursor_idx: usize,
    settings: &mut Settings,
) -> Option<crate::app::state::PopupType> {
    match cursor_idx {
        10 => {
            settings.editor_tab_size = match settings.editor_tab_size {
                2 => 4,
                4 => 8,
                _ => 2,
            };
        }
        21 => settings.viewer_use_external = !settings.viewer_use_external,
        26 => {
            settings.viewer_tab_size = match settings.viewer_tab_size {
                2 => 4,
                4 => 8,
                _ => 2,
            };
        }
        28 => settings.viewer_show_scrollbar = !settings.viewer_show_scrollbar,
        38 => settings.enter_use_external = !settings.enter_use_external,
        _ => {}
    }
    None
}
