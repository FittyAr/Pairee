use super::wrap_text;
use crate::config::localization::t;
use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier as StyleModifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
};

/// Strips the `.pairee` extension from a plugin name for display.
fn display_name(name: &str) -> &str {
    name.strip_suffix(".pairee").unwrap_or(name)
}

pub fn render_search(
    f: &mut Frame,
    list_area: Rect,
    detail_area: Rect,
    cursor_idx: usize,
    registry: &[(String, String, String, String)],
    is_searching: bool,
    editing_query: bool,
    theme: &crate::config::theme::Theme,
    border_style: Style,
    bg_style: Style,
) {
    let text_style = Style::default().fg(parse_color(&theme.popup_fg));
    let dim_style = Style::default().fg(Color::DarkGray);
    let bold_style = text_style.add_modifier(StyleModifier::BOLD);

    // Usable inner width (subtract 2 for borders, 1 leading space)
    let inner_w = (list_area.width as usize).saturating_sub(3);

    // Column widths: name takes the bulk, author fixed ~18, version fixed ~8
    let ver_w: usize = 8;
    let auth_w: usize = 18;
    let name_w: usize = inner_w.saturating_sub(ver_w + auth_w + 2); // 2 separators

    // ── Pagination ──────────────────────────────────────────────────────────
    // Leave 2 rows for borders and 1 for the page indicator at the bottom.
    let page_size = (list_area.height as usize).saturating_sub(3).max(1);
    let page = if registry.is_empty() {
        0
    } else {
        cursor_idx / page_size
    };
    let total_pages = if registry.is_empty() {
        1
    } else {
        (registry.len() + page_size - 1) / page_size
    };
    let slice_start = page * page_size;
    let slice_end = (slice_start + page_size).min(registry.len());

    let mut list_items: Vec<ListItem> = Vec::new();

    if is_searching {
        list_items.push(ListItem::new(Line::from(vec![Span::styled(
            t("plugin_search_searching"),
            Style::default().fg(Color::Yellow),
        )])));
    } else if registry.is_empty() {
        list_items.push(ListItem::new(Line::from(vec![Span::styled(
            t("plugin_search_no_results"),
            dim_style,
        )])));
    } else {
        // ── Column header ──────────────────────────────────────────────────
        let header = format!(
            " {:<name_w$}  {:<auth_w$}  {:<ver_w$}",
            "Plugin",
            "Author",
            "Version",
            name_w = name_w,
            auth_w = auth_w,
            ver_w = ver_w,
        );
        list_items.push(ListItem::new(Line::from(vec![Span::styled(
            header,
            dim_style.add_modifier(StyleModifier::UNDERLINED),
        )])));

        for (i, (name, version, _, author)) in registry[slice_start..slice_end].iter().enumerate() {
            let abs_idx = slice_start + i;
            let selected = abs_idx == cursor_idx;

            let style = if selected {
                Style::default()
                    .bg(parse_color(&theme.selection_bg))
                    .fg(parse_color(&theme.selection_fg))
                    .add_modifier(StyleModifier::BOLD)
            } else {
                text_style
            };

            let clean_name = display_name(name);

            // Truncate each column to its max width
            let name_col = truncate(clean_name, name_w);
            let auth_col = truncate(author, auth_w);
            let ver_col = truncate(version, ver_w);

            let row = format!(
                " {:<name_w$}  {:<auth_w$}  {:<ver_w$}",
                name_col,
                auth_col,
                ver_col,
                name_w = name_w,
                auth_w = auth_w,
                ver_w = ver_w,
            );

            list_items.push(ListItem::new(Line::from(vec![Span::styled(row, style)])));
        }

        // ── Page indicator ────────────────────────────────────────────────
        if total_pages > 1 {
            let indicator = format!(
                " {:>w$}",
                format!("Pg {}/{} — PgUp/PgDn", page + 1, total_pages),
                w = inner_w,
            );
            list_items.push(ListItem::new(Line::from(vec![Span::styled(
                indicator, dim_style,
            )])));
        }
    }

    // ── Hint: edit mode indicator in the block title ─────────────────────────
    let list_title = if editing_query {
        format!("{} [{}]", t("plugin_title"), t("plugin_search_typing"))
    } else {
        t("plugin_title")
    };

    let list_block = Block::default()
        .borders(Borders::ALL)
        .border_style(if editing_query {
            Style::default().fg(Color::Yellow)
        } else {
            border_style
        })
        .title(list_title)
        .style(bg_style);
    let list = List::new(list_items).block(list_block);
    f.render_widget(list, list_area);

    // ── Detail panel ─────────────────────────────────────────────────────────
    let detail_block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(t("plugin_details"))
        .style(bg_style);

    let mut detail_lines = Vec::new();
    if !registry.is_empty() {
        if let Some((name, version, desc, author)) = registry.get(cursor_idx) {
            detail_lines.push(Line::from(vec![
                Span::styled(t("plugin_detail_lbl"), bold_style),
                Span::styled(display_name(name).to_string(), text_style),
            ]));
            detail_lines.push(Line::from(vec![
                Span::styled(t("plugin_detail_latest_ver"), bold_style),
                Span::styled(version.clone(), text_style),
            ]));
            detail_lines.push(Line::from(vec![
                Span::styled(t("plugin_detail_author"), bold_style),
                Span::styled(author.clone(), text_style),
            ]));
            detail_lines.push(Line::from(""));
            detail_lines.push(Line::from(Span::styled(
                t("plugin_detail_description"),
                bold_style,
            )));
            let max_width = (detail_area.width as usize).saturating_sub(2);
            for line in wrap_text(desc, max_width) {
                detail_lines.push(Line::from(Span::styled(line, text_style)));
            }
        }
    } else {
        detail_lines.push(Line::from(Span::styled(t("plugin_no_selected"), dim_style)));
    }

    let detail_para = Paragraph::new(detail_lines)
        .block(detail_block)
        .wrap(Wrap { trim: false });
    f.render_widget(detail_para, detail_area);
}

/// Truncates a string to `max` visible characters, appending `…` if needed.
fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max.saturating_sub(1)])
    }
}
