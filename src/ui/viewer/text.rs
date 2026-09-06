use super::state::ViewerState;
use crate::ui::scrollbar::{self, ScrollTargetId, ScrollbarSurface, ScrollbarUiState};
use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph, Wrap},
};

pub(crate) fn highlight_line(
    line: &str,
    query: &str,
    case_sensitive: bool,
    normal_style: Style,
    highlight_style: Style,
) -> Vec<Span<'static>> {
    if query.is_empty() {
        return vec![Span::styled(line.to_string(), normal_style)];
    }

    let mut spans = Vec::new();
    let query_len = query.chars().count();
    let line_chars: Vec<char> = line.chars().collect();
    let line_len = line_chars.len();

    let query_lower: Vec<char> = if case_sensitive {
        query.chars().collect()
    } else {
        query.to_lowercase().chars().collect()
    };

    let line_lower: Vec<char> = if case_sensitive {
        line_chars.clone()
    } else {
        line.to_lowercase().chars().collect()
    };

    let mut i = 0;
    while i < line_len {
        let mut matches = false;
        if i + query_len <= line_lower.len() {
            matches = true;
            for j in 0..query_len {
                if line_lower[i + j] != query_lower[j] {
                    matches = false;
                    break;
                }
            }
        }

        if matches {
            let match_str: String = line_chars[i..i + query_len].iter().collect();
            spans.push(Span::styled(match_str, highlight_style));
            i += query_len;
        } else {
            let mut normal_str = String::new();
            normal_str.push(line_chars[i]);
            i += 1;

            while i < line_len {
                let mut sub_matches = false;
                if i + query_len <= line_lower.len() {
                    sub_matches = true;
                    for j in 0..query_len {
                        if line_lower[i + j] != query_lower[j] {
                            sub_matches = false;
                            break;
                        }
                    }
                }
                if sub_matches {
                    break;
                }
                normal_str.push(line_chars[i]);
                i += 1;
            }
            spans.push(Span::styled(normal_str, normal_style));
        }
    }

    spans
}

pub(crate) fn render_text(
    f: &mut Frame,
    area: Rect,
    state: &ViewerState,
    block: Block,
    theme: &crate::config::theme::Theme,
    active_popup: Option<&crate::app::state::PopupType>,
    show_scrollbar: bool,
    scrollbar: Option<&ScrollbarUiState>,
) {
    let height = area.height.saturating_sub(2) as usize;

    let search_info = match active_popup {
        Some(crate::app::state::PopupType::ViewerSearchPrompt {
            query,
            case_sensitive,
            ..
        }) if !query.is_empty() => Some((query.as_str(), *case_sensitive)),
        _ => None,
    };

    let lines: Vec<Line> = state
        .lines
        .iter()
        .skip(state.scroll)
        .take(height)
        .map(|l| {
            if let Some((q, cs)) = search_info {
                let normal_style = Style::default().fg(parse_color(&theme.panel_fg));
                let highlight_style = Style::default()
                    .bg(parse_color(&theme.selection_bg))
                    .fg(parse_color(&theme.marked_fg))
                    .add_modifier(Modifier::BOLD);
                Line::from(highlight_line(l, q, cs, normal_style, highlight_style))
            } else {
                Line::from(Span::raw(l.clone()))
            }
        })
        .collect();

    let para = Paragraph::new(lines)
        .block(block)
        .style(Style::default().fg(parse_color(&theme.panel_fg)))
        .wrap(Wrap { trim: false });

    f.render_widget(para, area);

    if show_scrollbar {
        scrollbar::render_vertical_inside_block(
            f,
            area,
            state.lines.len(),
            height,
            state.scroll,
            theme,
            ScrollbarSurface::Panel,
            scrollbar,
            ScrollTargetId::Viewer,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::highlight_line;
    use ratatui::style::Style;

    #[test]
    fn highlight_line_marks_query_case_insensitive() {
        let spans = highlight_line(
            "Hello HELLO",
            "hello",
            false,
            Style::default(),
            Style::default(),
        );
        assert!(spans.len() >= 2);
        let joined: String = spans.iter().map(|s| s.content.as_ref()).collect();
        assert_eq!(joined, "Hello HELLO");
    }

    #[test]
    fn highlight_line_empty_query_is_single_span() {
        let spans = highlight_line("abc", "", true, Style::default(), Style::default());
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].content.as_ref(), "abc");
    }
}
