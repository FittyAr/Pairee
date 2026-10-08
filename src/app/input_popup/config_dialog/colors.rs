use crate::app::context::AppContext;
use crate::app::state::PopupType;
use crate::config::settings::Settings;

pub fn handle_row(
    cursor_idx: usize,
    settings: &mut Settings,
    _editing_value: &mut bool,
    _edit_buffer: &mut String,
    context: &AppContext,
) -> Option<PopupType> {
    match cursor_idx {
        0 => {
            settings.theme = match settings.theme.as_str() {
                "slate" => "classic_blue".to_string(),
                _ => "slate".to_string(),
            };
            None
        }
        1 => Some(PopupType::ColorGroupsDialog {
            cursor_idx: 0,
            edit: None,
            theme: context.config.theme.clone(),
        }),
        2 => Some(PopupType::FilesHighlightingDialog {
            cursor_idx: 0,
            edit: None,
            rules: settings.highlight_rules.clone(),
        }),
        _ => None,
    }
}
