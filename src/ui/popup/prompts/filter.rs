use crate::app::state::PopupType;
use crate::config::localization::t;
use crate::ui::popup::kit::{self, TextBox};
use ratatui::{Frame, layout::Rect, style::Color};

/// Panel filter, quick filter and copy/move filter prompts: one mask field.
pub fn render(
    f: &mut Frame,
    popup: &PopupType,
    theme: &crate::config::theme::Theme,
    size: Rect,
) -> bool {
    let (input, title, text) = match popup {
        PopupType::FilePanelFilterPrompt { input }
        | PopupType::CopyMoveFilterPrompt { input, .. } => {
            (input, "prompt_filter_title", "prompt_filter_text")
        }
        PopupType::QuickFilterPrompt { input, .. } => (
            input,
            "prompt_quick_filter_title",
            "prompt_quick_filter_text",
        ),
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
