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
    if let PopupType::ConfirmOverwrite {
        src_paths,
        dest_dir,
        is_move,
        input,
    } = popup
    {
        let area = centered_rect_fixed(60, 9, size);
        f.render_widget(Clear, area);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Red))
            .title(t("prompt_overwrite_title"))
            .style(Style::default().bg(parse_color(&theme.popup_bg)));

        let op_name = if *is_move { t("op_move") } else { t("op_copy") };
        let first_name = src_paths
            .first()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        let target_desc = if src_paths.len() == 1 {
            if let Some(inp) = input {
                inp.clone()
            } else {
                first_name
            }
        } else {
            t("prompt_files_count").replacen("{}", &src_paths.len().to_string(), 1)
        };

        let text = t("prompt_overwrite_text")
            .replacen("{}", &dest_dir.to_string_lossy(), 1)
            .replacen("{}", &target_desc, 1)
            .replacen("{}", &op_name, 1);

        let paragraph = Paragraph::new(text)
            .block(block)
            .style(Style::default().fg(parse_color(&theme.popup_fg)));

        f.render_widget(paragraph, area);
        true
    } else {
        false
    }
}
