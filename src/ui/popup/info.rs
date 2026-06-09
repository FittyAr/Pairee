use super::centered_rect;
use crate::app::state::PopupType;
use crate::ui::theme_apply::parse_color;
use crate::config::localization::t;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

pub fn render_info_popup(
    f: &mut Frame,
    popup: &PopupType,
    theme: &crate::config::theme::Theme,
    size: Rect,
) -> bool {
    match popup {
        PopupType::InfoPanel { lines } => {
            let area = centered_rect(55, 55, size);
            f.render_widget(Clear, area);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(t("info_panel_title"))
                .style(Style::default().bg(parse_color(&theme.popup_bg)));

            let text_lines: Vec<Line> = lines
                .iter()
                .map(|l| Line::from(Span::raw(format!(" {}", l))))
                .collect();

            let paragraph = Paragraph::new(text_lines)
                .block(block)
                .style(Style::default().fg(parse_color(&theme.popup_fg)));

            f.render_widget(paragraph, area);
            true
        }
        PopupType::FileAttributesDialog { attrs, mode_input } => {
            let area = centered_rect(65, 45, size);
            f.render_widget(Clear, area);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(parse_color(&theme.popup_border)))
                .title(t("prompt_attributes_title"))
                .style(Style::default().bg(parse_color(&theme.popup_bg)));

            let path_str = attrs.path.to_string_lossy();
            let file_name = attrs
                .path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| path_str.to_string());
            let readonly_status = if attrs.readonly { t("info_yes") } else { t("info_no") };

            let format_time = |t_val: Option<std::time::SystemTime>| {
                t_val.map(|st| {
                    let datetime: chrono::DateTime<chrono::Local> = st.into();
                    datetime.format("%Y-%m-%d %H:%M:%S").to_string()
                })
                .unwrap_or_else(|| t("info_na"))
            };

            let modified_str = format_time(attrs.modified);
            let created_str = format_time(attrs.created);

            let text = t("info_attrs_text")
                .replacen("{}", &file_name, 1)
                .replacen("{}", &path_str, 1)
                .replacen("{}", &attrs.size.to_string(), 1)
                .replacen("{}", &attrs.owner, 1)
                .replacen("{}", &attrs.nlinks.to_string(), 1)
                .replacen("{}", &readonly_status, 1)
                .replacen("{}", &modified_str, 1)
                .replacen("{}", &created_str, 1)
                .replacen("{}", mode_input, 1);

            let paragraph = Paragraph::new(text)
                .block(block)
                .style(Style::default().fg(parse_color(&theme.popup_fg)));

            f.render_widget(paragraph, area);
            true
        }
        _ => false,
    }
}
