use crate::ui::popup::centered_rect_fixed;
use crate::app::state::PopupType;
use crate::config::localization::t;
use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, Clear, Paragraph},
};

pub fn render(
    f: &mut Frame,
    popup: &PopupType,
    theme: &crate::config::theme::Theme,
    size: Rect,
) -> bool {
    if let PopupType::DescribeFilePrompt {
        path,
        current_desc,
        input,
    } = popup
    {
        let area = centered_rect_fixed(60, 10, size);
        f.render_widget(Clear, area);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .title(t("prompt_description_title"))
            .style(Style::default().bg(parse_color(&theme.popup_bg)));

        let file_name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let text = t("prompt_describe_text")
            .replacen("{}", &file_name, 1)
            .replacen("{}", current_desc, 1)
            .replacen("{}", input, 1);

        let paragraph = Paragraph::new(text)
            .block(block)
            .style(Style::default().fg(parse_color(&theme.popup_fg)));

        f.render_widget(paragraph, area);
        true
    } else {
        false
    }
}
