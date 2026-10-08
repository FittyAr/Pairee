use crate::app::state::{LinkKind, PopupType};
use crate::config::localization::t;
use crate::ui::popup::kit::{self, TextBox};
use ratatui::{Frame, layout::Rect, style::Color};

pub fn render(
    f: &mut Frame,
    popup: &PopupType,
    theme: &crate::config::theme::Theme,
    size: Rect,
) -> bool {
    let PopupType::CreateLinkPrompt {
        src,
        dest_input,
        kind,
    } = popup
    else {
        return false;
    };
    let title = match kind {
        LinkKind::Symbolic => t("prompt_symlink_title"),
        LinkKind::Hard => t("prompt_hardlink_title"),
    };
    let template = t("prompt_link_text").replacen("{}", &crate::fs::file_name_lossy(src), 1);
    TextBox {
        size: (60, 9),
        title,
        border: kit::fg(Color::Yellow),
        body: kit::prompt_text(&template, dest_input, theme),
        body_style: kit::popup_fg(theme),
    }
    .render(f, size, theme);
    true
}
