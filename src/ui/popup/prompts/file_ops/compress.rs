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
    if let PopupType::CompressPrompt {
        input,
        targets,
        dest_dir,
    } = popup
    {
        let area = centered_rect_fixed(60, 9, size);
        f.render_widget(Clear, area);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow))
            .title(t("prompt_compress_title"))
            .style(Style::default().bg(parse_color(&theme.popup_bg)));

        let count = targets.len();
        let first_name = targets
            .first()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        let src_label = if count == 1 {
            t("prompt_compress_sing").replacen("{}", &first_name, 1)
        } else {
            t("prompt_compress_plur").replacen("{}", &count.to_string(), 1)
        };

        let text = format!(
            "\n {}\n {}\n\n > {}.zip\n\n {}",
            src_label,
            t("prompt_copy_dest").replacen("{}", &dest_dir.to_string_lossy(), 1),
            input,
            t("prompt_confirm_cancel_hint")
        );

        let paragraph = Paragraph::new(text)
            .block(block)
            .style(Style::default().fg(parse_color(&theme.popup_fg)));

        f.render_widget(paragraph, area);
        true
    } else {
        false
    }
}
