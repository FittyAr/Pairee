use crate::app::input::panel_find::NameMatch;
use crate::app::state::PopupType;
use crate::config::localization::t;
use crate::ui::popup::kit::{self, TextBox};
use ratatui::{Frame, layout::Rect, style::Color};

/// One-field prompts: panel filter, quick filter, copy/move filter, the
/// tab title and the in-panel search.
pub fn render(
    f: &mut Frame,
    popup: &PopupType,
    theme: &crate::config::theme::Theme,
    size: Rect,
) -> bool {
    let (input, title, text) = match popup {
        PopupType::PanelSearch { query, how, .. } => match how {
            NameMatch::Substring => (
                query,
                "prompt_panel_search_title",
                "prompt_panel_search_text",
            ),
            NameMatch::Prefix => (
                query,
                "prompt_quick_search_title",
                "prompt_quick_search_text",
            ),
        },
        PopupType::FilePanelFilterPrompt { input }
        | PopupType::CopyMoveFilterPrompt { input, .. } => {
            (input, "prompt_filter_title", "prompt_filter_text")
        }
        PopupType::QuickFilterPrompt { input, .. } => (
            input,
            "prompt_quick_filter_title",
            "prompt_quick_filter_text",
        ),
        PopupType::RenameTabPrompt { input, .. } => {
            (input, "prompt_rename_tab_title", "prompt_rename_tab_text")
        }
        _ => return false,
    };
    TextBox {
        size: (50, 9),
        title: t(title),
        border: kit::fg(Color::Cyan),
        body: kit::prompt_text(&t(text), input, theme),
        body_style: kit::popup_fg(theme),
    }
    .render(f, size, theme);
    true
}
