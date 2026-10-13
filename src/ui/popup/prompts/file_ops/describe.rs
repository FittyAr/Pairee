use crate::app::state::PopupType;
use crate::config::localization::t;
use crate::ui::popup::kit::{self, TextBox};
use ratatui::{Frame, layout::Rect, style::Color};

pub fn render(
    f: &mut Frame,
    popup: &PopupType,
    theme: &crate::ui::resolved_theme::ResolvedTheme,
    size: Rect,
) -> bool {
    let PopupType::DescribeFilePrompt {
        path,
        current_desc,
        input,
    } = popup
    else {
        return false;
    };
    let template = t("prompt_describe_text")
        .replacen("{}", &crate::fs::file_name_lossy(path), 1)
        .replacen("{}", current_desc, 1);
    TextBox {
        size: (60, 10),
        title: t("prompt_description_title"),
        border: kit::fg(Color::Cyan),
        body: kit::prompt_text(&template, input, theme),
        body_style: theme.popup_fg(),
    }
    .with_field(kit::field_at(&template))
    .render(f, size, theme);
    true
}
