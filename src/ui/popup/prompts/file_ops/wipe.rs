use crate::app::state::PopupType;
use crate::config::localization::t;
use crate::ui::popup::centered_rect_fixed;
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
    if let PopupType::WipeConfirm { paths } = popup {
        let area = centered_rect_fixed(55, 8, size);
        f.render_widget(Clear, area);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Red))
            .title(t("prompt_wipe_warn_title"))
            .style(Style::default().bg(parse_color(&theme.popup_bg)));

        let text = t("prompt_wipe_warn_text").replacen("{}", &paths.len().to_string(), 1);

        let paragraph = Paragraph::new(text)
            .block(block)
            .style(Style::default().fg(Color::LightRed));

        f.render_widget(paragraph, area);
        true
    } else {
        false
    }
}
