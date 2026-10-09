use crate::app::state::PopupType;
use crate::config::localization::t;
use crate::ui::popup::kit;
use ratatui::{Frame, layout::Rect, style::Color};

pub fn render(
    f: &mut Frame,
    popup: &PopupType,
    theme: &crate::config::theme::Theme,
    size: Rect,
) -> bool {
    let PopupType::WipeConfirm { paths } = popup else {
        return false;
    };
    kit::TextBox {
        size: (55, 8),
        title: t("prompt_wipe_warn_title"),
        border: kit::fg(Color::Red),
        body: t("prompt_wipe_warn_text")
            .replacen("{}", &paths.len().to_string(), 1)
            .into(),
        body_style: kit::fg(Color::LightRed),
    }
    .render(f, size, theme);
    true
}
