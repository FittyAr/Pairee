use crate::ui::popup::centered_rect_fixed;
use crate::app::state::{PopupType, LinkKind};
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
    if let PopupType::CreateLinkPrompt {
        src,
        dest_input,
        kind,
    } = popup
    {
        let area = centered_rect_fixed(60, 9, size);
        f.render_widget(Clear, area);

        let title = match kind {
            LinkKind::Symbolic => t("prompt_symlink_title"),
            LinkKind::Hard => t("prompt_hardlink_title"),
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow))
            .title(title)
            .style(Style::default().bg(parse_color(&theme.popup_bg)));

        let src_name = src
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let text = t("prompt_link_text")
            .replacen("{}", &src_name, 1)
            .replacen("{}", dest_input, 1);

        let paragraph = Paragraph::new(text)
            .block(block)
            .style(Style::default().fg(parse_color(&theme.popup_fg)));

        f.render_widget(paragraph, area);
        true
    } else {
        false
    }
}
