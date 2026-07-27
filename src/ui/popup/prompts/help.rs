use super::super::centered_rect;
use crate::app::state::PopupType;
use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{
        Block, Borders, Clear, List, ListItem, ListState, Padding, Paragraph, Scrollbar,
        ScrollbarOrientation, ScrollbarState,
    },
};
use std::path::{Path, PathBuf};

/// Returns the Diátaxis quadrant for a help doc path, or `"Plugins"` for
/// anything that doesn't follow the `NN_<quadrant>_…` filename convention.
///
/// The two-digit prefix on each core doc file already encodes the quadrant
/// (00 = index, 10-19 = tutorial, 20-39 = how-to, 40-49 = reference,
/// 50-59 = explanation). Plugin docs use the bare locale code as their
/// stem (`en`, `es`, …) and land in a single fallback group.
fn category_from_path(path: &Path) -> &'static str {
    let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
        return "Plugins";
    };
    let prefix = stem.split('_').next().unwrap_or("");
    if prefix.len() != 2 || !prefix.chars().all(|c| c.is_ascii_digit()) {
        return "Plugins";
    }
    // Unwrap is safe: the previous `all(is_ascii_digit)` check guarantees parse success.
    match prefix.parse::<u32>().unwrap_or(99) {
        0..=9 => "Index",
        10..=19 => "Tutorials",
        20..=39 => "How-To",
        40..=49 => "Reference",
        50..=59 => "Explanation",
        _ => "Plugins",
    }
}

/// Build the rendered rows for the help list, inserting a category header
/// row whenever the category changes between consecutive docs.
///
/// `cursor_idx` indexes into `current_docs`; the returned `doc_to_row`
/// vector maps each doc index to its row index in the rendered list, so
/// `ListState` can highlight the right row even though headers occupy
/// intermediate positions.
fn build_help_rows(
    current_docs: &[(String, PathBuf)],
    cursor_idx: usize,
    popup_bg: Color,
    popup_fg: Color,
    selection_bg: Color,
    selection_fg: Color,
) -> (Vec<ListItem<'static>>, Vec<usize>) {
    let mut rows: Vec<ListItem<'static>> = Vec::with_capacity(current_docs.len() + 4);
    let mut doc_to_row: Vec<usize> = Vec::with_capacity(current_docs.len());
    let mut prev_category: Option<&'static str> = None;

    for (doc_idx, (title, path)) in current_docs.iter().enumerate() {
        let category = category_from_path(path);
        if Some(category) != prev_category {
            rows.push(make_category_header(category, popup_fg, popup_bg));
            prev_category = Some(category);
        }
        doc_to_row.push(rows.len());
        rows.push(make_doc_row(
            title,
            doc_idx == cursor_idx,
            popup_fg,
            popup_bg,
            selection_bg,
            selection_fg,
        ));
    }

    (rows, doc_to_row)
}

fn make_category_header(category: &str, popup_fg: Color, popup_bg: Color) -> ListItem<'static> {
    // Section headers are bold, uppercase, and rendered in a muted tone so
    // they sit visually behind the selectable items (interface-design:
    // "weight and color do more hierarchy work than size").
    let label = format!(" {} ", category.to_uppercase());
    let header_style = Style::default()
        .fg(popup_fg)
        .add_modifier(Modifier::BOLD | Modifier::DIM);
    let line = ratatui::text::Line::from(vec![ratatui::text::Span::styled(
        label,
        header_style,
    )]);
    ListItem::new(line).style(Style::default().bg(popup_bg))
}

fn make_doc_row(
    title: &str,
    is_selected: bool,
    popup_fg: Color,
    popup_bg: Color,
    selection_bg: Color,
    selection_fg: Color,
) -> ListItem<'static> {
    // P3: selection gets a row-wide background via ListItem::style, not just
    // a Span-level style on the text. This way the highlight fills the full
    // row even when the title is short.
    let (row_bg, text_style) = if is_selected {
        (
            selection_bg,
            Style::default()
                .fg(selection_fg)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        (popup_bg, Style::default().fg(popup_fg))
    };
    let line = ratatui::text::Line::from(vec![ratatui::text::Span::styled(
        format!("  {}  ", title),
        text_style,
    )]);
    ListItem::new(line).style(Style::default().bg(row_bg))
}

fn render_tab_bar(
    f: &mut Frame,
    area: Rect,
    active_tab: usize,
    mode: usize,
    border_color: Color,
    popup_bg: Color,
) {
    use ratatui::text::{Line, Span};

    let tab_title_core = "Core Help";
    let tab_title_plugins = "Plugins Help";

    let core_style = if active_tab == 0 {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let plugins_style = if active_tab == 1 {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let tabs_line = Line::from(vec![
        Span::styled(" [ ", Style::default().fg(Color::DarkGray)),
        Span::styled(tab_title_core, core_style),
        Span::styled(" ]  [ ", Style::default().fg(Color::DarkGray)),
        Span::styled(tab_title_plugins, plugins_style),
        Span::styled(" ]", Style::default().fg(Color::DarkGray)),
    ]);

    let border_color = if mode == 0 { Color::Yellow } else { border_color };
    let block = Block::default()
        .borders(Borders::TOP | Borders::LEFT | Borders::RIGHT)
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(popup_bg));
    f.render_widget(Paragraph::new(tabs_line).block(block), area);
}

pub fn render(
    f: &mut Frame,
    popup: &PopupType,
    theme: &crate::config::theme::Theme,
    size: Rect,
) -> bool {
    let PopupType::Help {
        mode,
        docs,
        plugin_docs,
        active_tab,
        cursor_idx,
        scroll_y,
        active_content,
    } = popup
    else {
        return false;
    };

    let popup_area = centered_rect(90, 85, size);
    f.render_widget(Clear, popup_area);

    // P1: reserve the last row of the popup for the hint, so the list and
    // content viewer never overlap with it. The previous version placed
    // the hint at `area.y + area.height - 2`, which collided with the
    // bottom border of the list block and the last content line.
    let v_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(popup_area);
    let main_area = v_chunks[0];
    let hint_area = v_chunks[1];

    // P4: widen the document list from 25% to 30% so titles like
    // "Tutorial: Panels and Screens" stop truncating. The 30/70 split
    // still declares "the list serves the content" (proportions speak).
    let h_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(main_area);
    let left_area = h_chunks[0];
    let right_area = h_chunks[1];

    // Theme tokens, hoisted to avoid repeated parsing inside the loop.
    let popup_bg = parse_color(&theme.popup_bg);
    let popup_fg = parse_color(&theme.popup_fg);
    let selection_bg = parse_color(&theme.selection_bg);
    let selection_fg = parse_color(&theme.selection_fg);
    let popup_border = parse_color(&theme.popup_border);

    // ── Left panel: tab bar + grouped document list ──────────────────────
    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(1)])
        .split(left_area);
    let tab_area = left_chunks[0];
    let list_area = left_chunks[1];

    render_tab_bar(f, tab_area, *active_tab, *mode, popup_border, popup_bg);

    let current_docs: &[(String, PathBuf)] = if *active_tab == 0 { docs } else { plugin_docs };

    // P2: build the rendered rows with category headers interleaved. The
    // list is already sorted alphabetically in `ui_settings.rs`, and the
    // filename prefix convention (00_, 10_, 20_, 40_, 50_) puts the
    // Diátaxis quadrants in the right order, so the result is grouped by
    // section without any extra sort.
    let (list_items, doc_to_row) = build_help_rows(
        current_docs,
        *cursor_idx,
        popup_bg,
        popup_fg,
        selection_bg,
        selection_fg,
    );

    let left_border_color = if *mode == 0 { Color::Yellow } else { popup_border };
    let left_block = Block::default()
        .borders(Borders::BOTTOM | Borders::LEFT | Borders::RIGHT)
        .border_style(Style::default().fg(left_border_color))
        .style(Style::default().bg(popup_bg));

    // The list_state points at the rendered row, not at the doc index, so
    // the selection highlight lines up with the right item even though
    // category headers occupy intermediate rows.
    let mut list_state = ListState::default();
    if let Some(&row_idx) = doc_to_row.get(*cursor_idx) {
        list_state.select(Some(row_idx));
    }

    let list = List::new(list_items)
        .block(left_block)
        .style(Style::default().bg(popup_bg));
    f.render_stateful_widget(list, list_area, &mut list_state);

    // Scrollbar on the right edge when the list overflows the inner area.
    // The block has BOTTOM|LEFT|RIGHT borders, so the inner height is
    // `list_area.height - 1` (just the bottom border).
    let list_inner_height = list_area.height.saturating_sub(1) as usize;
    // The list's last row index is one past the last doc's row index; the
    // scrollbar's content length is that index, capped by the inner height.
    let list_last_row = doc_to_row.last().copied().unwrap_or(0).saturating_add(1);
    if list_inner_height > 0 && list_last_row > list_inner_height {
        let max_offset = list_last_row.saturating_sub(list_inner_height);
        let mut scrollbar_state = ScrollbarState::new(max_offset)
            .position(list_state.offset().min(max_offset));
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight);
        let scrollbar_area = Rect {
            x: left_area.x + left_area.width.saturating_sub(1),
            y: list_area.y,
            width: 1,
            height: list_area.height.saturating_sub(1),
        };
        f.render_stateful_widget(scrollbar, scrollbar_area, &mut scrollbar_state);
    }

    // ── Right panel: content viewer with inner padding (P5) ──────────────
    let doc_title = current_docs
        .get(*cursor_idx)
        .map(|(t, _)| t.as_str())
        .unwrap_or(" Documentation ");
    let right_border_color = if *mode == 1 { Color::Yellow } else { popup_border };
    let right_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(right_border_color))
        .title(format!(" {} ", doc_title))
        .style(Style::default().bg(popup_bg))
        // P5: give the text room to breathe — 1 col on each side keeps the
        // text off the border without making the panel feel padded.
        .padding(Padding::horizontal(1));

    if let Some(content) = active_content {
        let parsed_lines = parse_markdown_to_lines(content);
        let inner_width = (right_area.width.saturating_sub(4)) as usize;
        let wrapped_lines = wrap_lines(parsed_lines, inner_width);

        let paragraph = Paragraph::new(wrapped_lines.clone())
            .block(right_block)
            .scroll((*scroll_y as u16, 0))
            .style(Style::default().fg(popup_fg));
        f.render_widget(paragraph, right_area);

        // Scrollbar when the wrapped text overflows the panel.
        let total_lines = wrapped_lines.len();
        let inner_height = right_area.height.saturating_sub(2) as usize;
        if total_lines > inner_height {
            let mut scrollbar_state =
                ScrollbarState::new(total_lines.saturating_sub(inner_height))
                    .position((*scroll_y).min(total_lines.saturating_sub(inner_height)));
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight);
            let scrollbar_area = Rect {
                x: right_area.x + right_area.width.saturating_sub(1),
                y: right_area.y + 1,
                width: 1,
                height: right_area.height.saturating_sub(2),
            };
            f.render_stateful_widget(scrollbar, scrollbar_area, &mut scrollbar_state);
        }
    } else {
        let empty_paragraph = Paragraph::new(" No document loaded ").block(right_block);
        f.render_widget(empty_paragraph, right_area);
    }

    // Hint row, full width of the popup, below the main content.
    let hint_text = " [Tab] Switch Panels  [Up/Down/j/k] Navigate/Scroll  [Esc] Close ";
    f.render_widget(
        Paragraph::new(hint_text)
            .alignment(ratatui::layout::Alignment::Center)
            .style(Style::default().fg(Color::DarkGray)),
        hint_area,
    );
    true
}

fn parse_markdown_to_lines(text: &str) -> Vec<ratatui::text::Line<'static>> {
    use pulldown_cmark::{Event, HeadingLevel, Parser, Tag, TagEnd};
    use ratatui::style::{Color, Modifier, Style};
    use ratatui::text::{Line, Span};

    let parser = Parser::new(text);
    let mut lines = Vec::new();
    let mut current_spans = Vec::new();

    let mut bold = false;
    let mut italic = false;
    let code = false;
    let mut link = false;

    for event in parser {
        match event {
            Event::Start(tag) => match tag {
                Tag::Heading { level, .. } => {
                    if !current_spans.is_empty() {
                        lines.push(Line::from(current_spans.drain(..).collect::<Vec<_>>()));
                    }
                    if !lines.is_empty() {
                        lines.push(Line::from(""));
                    }

                    let prefix = match level {
                        HeadingLevel::H1 => "# ",
                        HeadingLevel::H2 => "## ",
                        HeadingLevel::H3 => "### ",
                        _ => "#### ",
                    };
                    current_spans.push(Span::styled(
                        prefix,
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    ));
                }
                Tag::Paragraph => {
                    if !current_spans.is_empty() {
                        lines.push(Line::from(current_spans.drain(..).collect::<Vec<_>>()));
                    }
                }
                Tag::Emphasis => italic = true,
                Tag::Strong => bold = true,
                Tag::Link { .. } => link = true,
                Tag::Item => {
                    if !current_spans.is_empty() {
                        lines.push(Line::from(current_spans.drain(..).collect::<Vec<_>>()));
                    }
                    current_spans.push(Span::styled("• ", Style::default().fg(Color::Cyan)));
                }
                _ => {}
            },
            Event::End(tag) => match tag {
                TagEnd::Heading(_) => {
                    if !current_spans.is_empty() {
                        for span in &mut current_spans {
                            span.style = span.style.fg(Color::Yellow).add_modifier(Modifier::BOLD);
                        }
                        lines.push(Line::from(current_spans.drain(..).collect::<Vec<_>>()));
                    }
                    lines.push(Line::from(""));
                }
                TagEnd::Paragraph => {
                    if !current_spans.is_empty() {
                        lines.push(Line::from(current_spans.drain(..).collect::<Vec<_>>()));
                    }
                    lines.push(Line::from(""));
                }
                TagEnd::Emphasis => italic = false,
                TagEnd::Strong => bold = false,
                TagEnd::Link => link = false,
                TagEnd::Item => {
                    if !current_spans.is_empty() {
                        lines.push(Line::from(current_spans.drain(..).collect::<Vec<_>>()));
                    }
                }
                _ => {}
            },
            Event::Text(t) => {
                let mut style = Style::default();
                if bold {
                    style = style.add_modifier(Modifier::BOLD);
                }
                if italic {
                    style = style.add_modifier(Modifier::ITALIC);
                }
                if code {
                    style = style.fg(Color::Magenta);
                } else if link {
                    style = style.fg(Color::Blue).add_modifier(Modifier::UNDERLINED);
                } else {
                    style = style.fg(Color::White);
                }
                current_spans.push(Span::styled(t.into_string(), style));
            }
            Event::Code(c) => {
                current_spans.push(Span::styled(
                    format!(" `{}` ", c),
                    Style::default().fg(Color::Magenta),
                ));
            }
            Event::SoftBreak | Event::HardBreak => {
                if !current_spans.is_empty() {
                    lines.push(Line::from(current_spans.drain(..).collect::<Vec<_>>()));
                }
            }
            _ => {}
        }
    }

    if !current_spans.is_empty() {
        lines.push(Line::from(current_spans.drain(..).collect::<Vec<_>>()));
    }

    lines
}

fn wrap_lines(
    lines: Vec<ratatui::text::Line<'static>>,
    width: usize,
) -> Vec<ratatui::text::Line<'static>> {
    let mut wrapped = Vec::new();
    for line in lines {
        let total_chars: usize = line.spans.iter().map(|s| s.content.chars().count()).sum();
        if total_chars <= width {
            wrapped.push(line);
            continue;
        }

        let mut current_line_spans = Vec::new();
        let mut current_width = 0;

        for span in line.spans {
            let text = span.content;
            let style = span.style;

            let mut words = Vec::new();
            let mut word = String::new();
            for c in text.chars() {
                if c.is_whitespace() {
                    if !word.is_empty() {
                        words.push((word.clone(), false));
                        word.clear();
                    }
                    words.push((c.to_string(), true));
                } else {
                    word.push(c);
                }
            }
            if !word.is_empty() {
                words.push((word, false));
            }

            for (w, is_space) in words {
                let w_len = w.chars().count();
                if current_width + w_len > width && !is_space && current_width > 0 {
                    wrapped.push(ratatui::text::Line::from(current_line_spans));
                    current_line_spans = Vec::new();
                    current_width = 0;
                }

                if w_len > width {
                    let chars: Vec<char> = w.chars().collect();
                    for chunk in chars.chunks(width) {
                        let chunk_str: String = chunk.iter().collect();
                        wrapped.push(ratatui::text::Line::from(vec![
                            ratatui::text::Span::styled(chunk_str, style),
                        ]));
                    }
                    continue;
                }

                current_line_spans.push(ratatui::text::Span::styled(w, style));
                current_width += w_len;
            }
        }
        if !current_line_spans.is_empty() {
            wrapped.push(ratatui::text::Line::from(current_line_spans));
        }
    }
    wrapped
}
