use crate::app::state::PopupType;
use crate::config::localization::t;
use crate::ui::popup::centered_rect;
use crate::ui::scrollbar::ScrollbarUiState;
use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

pub(super) fn render_search(
    f: &mut Frame,
    popup: &PopupType,
    theme: &crate::config::theme::Theme,
    size: Rect,
    _scrollbar: Option<&ScrollbarUiState>,
) -> bool {
    match popup {
        PopupType::SearchPrompt {
            query,
            content_query,
            search_root,
            case_sensitive,
            search_target,
            cursor_idx,
        } => {
            use ratatui::layout::{Constraint, Direction, Layout};

            let area = centered_rect(65, 40, size);
            f.render_widget(Clear, area);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(t("prompt_search_title"))
                .style(Style::default().bg(parse_color(&theme.popup_bg)));

            let inner = block.inner(area);
            f.render_widget(block, area);

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1), // 0: Search in root folder
                    Constraint::Length(2), // 1: File name pattern
                    Constraint::Length(2), // 2: Content query
                    Constraint::Length(1), // 3: Separator line
                    Constraint::Length(1), // 4: Case sensitive
                    Constraint::Length(1), // 5: Search target
                    Constraint::Min(1),    // 6: Spacer
                    Constraint::Length(1), // 7: Buttons
                ])
                .split(inner);

            let act_style = Style::default().bg(Color::Cyan).fg(Color::Black);
            let norm_style = Style::default().fg(parse_color(&theme.popup_fg));

            // Search root path
            let root_str = search_root.to_string_lossy();
            f.render_widget(
                Paragraph::new(format!(" {}: {}", t("prompt_find_folder"), root_str))
                    .style(norm_style),
                chunks[0],
            );

            // File name pattern
            let q_pref = if *cursor_idx == 0 { "► " } else { "  " };
            let q_style = if *cursor_idx == 0 {
                act_style
            } else {
                norm_style
            };
            let q_text = if *cursor_idx == 0 {
                format!("{}_", query)
            } else {
                query.clone()
            };
            f.render_widget(
                Paragraph::new(format!(
                    "{}{}\n   > {}",
                    q_pref,
                    t("prompt_find_pattern"),
                    q_text
                ))
                .style(q_style),
                chunks[1],
            );

            // Content query
            let c_pref = if *cursor_idx == 1 { "► " } else { "  " };
            let c_style = if *cursor_idx == 1 {
                act_style
            } else {
                norm_style
            };
            let c_text = if *cursor_idx == 1 {
                format!("{}_", content_query)
            } else {
                content_query.clone()
            };
            f.render_widget(
                Paragraph::new(format!(
                    "{}{}\n   > {}",
                    c_pref,
                    t("prompt_find_content"),
                    c_text
                ))
                .style(c_style),
                chunks[2],
            );

            // Separator line
            let sep_style = Style::default().fg(Color::Cyan);
            let sep_str = ratatui::symbols::line::HORIZONTAL.repeat(inner.width as usize);
            f.render_widget(Paragraph::new(sep_str).style(sep_style), chunks[3]);

            // Case sensitive checkbox
            let cs_pref = if *cursor_idx == 2 { "► " } else { "  " };
            let cs_style = if *cursor_idx == 2 {
                act_style
            } else {
                norm_style
            };
            let cs_chk = if *case_sensitive { "[x]" } else { "[ ]" };
            f.render_widget(
                Paragraph::new(format!("{}{} {}", cs_pref, cs_chk, t("sys_case_sensitive")))
                    .style(cs_style),
                chunks[4],
            );

            // Search target selection
            let target_pref = if *cursor_idx == 3 { "► " } else { "  " };
            let target_style = if *cursor_idx == 3 {
                act_style
            } else {
                norm_style
            };
            let target_val = match search_target {
                crate::fs::search::SearchTarget::Any => t("search_target_any"),
                crate::fs::search::SearchTarget::File => t("search_target_file"),
                crate::fs::search::SearchTarget::Directory => t("search_target_dir"),
            };
            f.render_widget(
                Paragraph::new(format!(
                    "{}{} < {} >",
                    target_pref,
                    t("search_target_label"),
                    target_val
                ))
                .style(target_style),
                chunks[5],
            );

            // Buttons
            let b1 = if *cursor_idx == 4 {
                act_style
            } else {
                norm_style
            };
            let b2 = if *cursor_idx == 5 {
                act_style
            } else {
                norm_style
            };
            let btns = ratatui::text::Line::from(vec![
                ratatui::text::Span::styled(t("btn_ok_bracket"), b1),
                ratatui::text::Span::raw("  "),
                ratatui::text::Span::styled(t("btn_cancel_bracket"), b2),
            ]);
            f.render_widget(
                Paragraph::new(btns).alignment(ratatui::layout::Alignment::Center),
                chunks[7],
            );

            true
        }
        PopupType::SearchResults {
            query,
            results,
            cursor_idx,
            searching,
        } => {
            let area = centered_rect(70, 60, size);
            f.render_widget(Clear, area);

            let mut title = t("search_results_title").replacen("{}", query, 1).replacen(
                "{}",
                &results.len().to_string(),
                1,
            );
            if *searching {
                title.push_str(&t("searching_suffix"));
            }
            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(title)
                .style(Style::default().bg(parse_color(&theme.popup_bg)));

            let inner = block.inner(area);
            f.render_widget(block, area);

            if results.is_empty() {
                let text = if *searching {
                    t("searching_placeholder")
                } else {
                    t("search_results_empty")
                };
                let paragraph =
                    Paragraph::new(text).style(Style::default().fg(parse_color(&theme.popup_fg)));
                f.render_widget(paragraph, inner);
            } else {
                let list_height = inner.height.saturating_sub(2) as usize;
                let scroll_start = cursor_idx.saturating_sub(list_height / 2);
                let mut lines = Vec::new();

                for (i, (path, is_dir)) in results
                    .iter()
                    .enumerate()
                    .skip(scroll_start)
                    .take(list_height)
                {
                    let is_cursor = i == *cursor_idx;
                    let display = path.to_string_lossy().to_string();
                    let prefix = if *is_dir { "📁 " } else { "📄 " };
                    let display_str = format!("{} {}", prefix, display);

                    let style = if is_cursor {
                        Style::default()
                            .bg(parse_color(&theme.selection_bg))
                            .fg(parse_color(&theme.selection_fg))
                            .add_modifier(Modifier::BOLD)
                    } else if *is_dir {
                        Style::default().fg(Color::LightBlue)
                    } else {
                        Style::default().fg(parse_color(&theme.popup_fg))
                    };
                    lines.push(Line::from(Span::styled(
                        format!(" {} ", display_str),
                        style,
                    )));
                }

                let hint = Line::from(Span::styled(
                    t("search_results_hint"),
                    Style::default().fg(Color::DarkGray),
                ));
                lines.push(Line::from(""));
                lines.push(hint);

                let paragraph =
                    Paragraph::new(lines).style(Style::default().fg(parse_color(&theme.popup_fg)));
                f.render_widget(paragraph, inner);
            }
            true
        }
        _ => false,
    }
}
