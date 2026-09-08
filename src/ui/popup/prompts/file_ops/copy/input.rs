use crate::config::localization::t;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    widgets::Paragraph,
};
use std::path::PathBuf;

pub fn render_input(
    f: &mut Frame,
    area: Rect,
    src_paths: &[PathBuf],
    input: &str,
    cursor_idx: usize,
    act_style: Style,
    norm_style: Style,
    is_move: bool,
) {
    let count = src_paths.len();
    let first_name = src_paths
        .first()
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let label = if is_move {
        if count == 1 {
            t("prompt_move_sing").replacen("{}", &first_name, 1)
        } else {
            t("prompt_move_plur").replacen("{}", &count.to_string(), 1)
        }
    } else {
        if count == 1 {
            t("prompt_copy_sing").replacen("{}", &first_name, 1)
        } else {
            t("prompt_copy_plur").replacen("{}", &count.to_string(), 1)
        }
    };
    let in_style = if cursor_idx == 0 {
        act_style
    } else {
        norm_style
    };
    let to_text = if is_move {
        t("prompt_move_to")
    } else {
        t("prompt_copy_to")
    };
    let mut text_lines = vec![ratatui::text::Line::from(format!("{} {}", label, to_text))];
    let mut input_spans = vec![ratatui::text::Span::styled(input.to_string(), in_style)];
    if cursor_idx == 0 {
        input_spans.push(ratatui::text::Span::styled(
            "_",
            Style::default().fg(Color::Cyan),
        ));
        if !input.is_empty() {
            let history = crate::fs::transfer::history::load_history();
            if let Some(suggestion) = history
                .destinations
                .iter()
                .find(|d| d.to_lowercase().starts_with(&input.to_lowercase()))
                && suggestion.len() > input.len()
            {
                let suffix = &suggestion[input.len()..];
                input_spans.push(ratatui::text::Span::styled(
                    suffix.to_string(),
                    Style::default().fg(Color::DarkGray),
                ));
            }
        }
    }
    text_lines.push(ratatui::text::Line::from(input_spans));
    f.render_widget(Paragraph::new(ratatui::text::Text::from(text_lines)), area);
}
