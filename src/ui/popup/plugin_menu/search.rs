use super::wrap_text;
use crate::app::input_popup::plugin_menu::search::InstallStatus;
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

/// Short text marker rendered next to the version column to convey
/// the install status of the entry. Kept tiny (1-2 cells) so it
/// doesn't push the existing columns around — the existing column
/// widths already account for it.
fn status_marker(status: InstallStatus) -> (&'static str, Color) {
    match status {
        // Plugin is not installed: no marker, no clutter.
        InstallStatus::NotInstalled => ("", Color::Reset),
        // Installed and at the latest version: dim check-mark.
        InstallStatus::Installed => ("✓", Color::Green),
        // Installed but a newer version is available: bright arrow.
        InstallStatus::UpdateAvailable => ("↑", Color::Yellow),
    }
}

pub fn render_search(
    f: &mut Frame,
    list_area: Rect,
    detail_area: Rect,
    cursor_idx: usize,
    registry: &[(String, String, String, String)],
    installed: &[(String, String, bool, bool, Option<String>)],
    install_in_progress: Option<&str>,
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

    // Column widths: name takes the bulk, author fixed ~18, version
    // fixed ~8, status marker fixed 1 cell ("✓" or "↑").
    let marker_w: usize = 1;
    let ver_w: usize = 8;
    let auth_w: usize = 18;
    let name_w: usize = inner_w.saturating_sub(ver_w + auth_w + marker_w + 3); // 3 separators

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
            " {:<name_w$}  {:<auth_w$}  {:<ver_w$}  {}",
            "Plugin",
            "Author",
            "Version",
            "",
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
            let status = crate::app::input_popup::plugin_menu::search::install_status(
                name, version, installed,
            );
            let (marker, marker_color) = status_marker(status);
            let in_flight = install_in_progress == Some(name.as_str());

            // Truncate each column to its max width
            let name_col = truncate(clean_name, name_w);
            let auth_col = truncate(author, auth_w);
            let ver_col = truncate(version, ver_w);

            // Prefix the row with "⏳" while this entry is being
            // installed so the user can see which row is in flight
            // (the install task also emits a toast, but the row
            // marker stays until the install actually completes).
            let row_prefix = if in_flight { "⏳ " } else { "  " };

            let mut spans = vec![Span::styled(
                format!(
                    "{}{:<name_w$}  {:<auth_w$}  {:<ver_w$}  ",
                    row_prefix,
                    name_col,
                    auth_col,
                    ver_col,
                    name_w = name_w,
                    auth_w = auth_w,
                    ver_w = ver_w,
                ),
                style,
            )];
            if !marker.is_empty() {
                spans.push(Span::styled(
                    marker.to_string(),
                    style.fg(marker_color).add_modifier(StyleModifier::BOLD),
                ));
            }

            list_items.push(ListItem::new(Line::from(spans)));
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

            // Show the locally installed version (if any) and the
            // current install status so the user knows whether the
            // action bound to `i` is going to install, reinstall,
            // or update.
            let status = crate::app::input_popup::plugin_menu::search::install_status(
                name, version, installed,
            );
            match status {
                InstallStatus::NotInstalled => {
                    detail_lines.push(Line::from(vec![
                        Span::styled(format!("{}: ", t("plugin_detail_local_ver")), bold_style),
                        Span::styled(t("plugin_detail_local_none"), dim_style),
                    ]));
                }
                InstallStatus::Installed | InstallStatus::UpdateAvailable => {
                    if let Some((_, installed_version, _, _, _)) =
                        installed.iter().find(|(n, _, _, _, _)| n == name)
                    {
                        detail_lines.push(Line::from(vec![
                            Span::styled(format!("{}: ", t("plugin_detail_local_ver")), bold_style),
                            Span::styled(installed_version.clone(), text_style),
                        ]));
                    }
                }
            }
            if install_in_progress == Some(name.as_str()) {
                detail_lines.push(Line::from(Span::styled(
                    t("plugin_detail_install_in_progress"),
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(StyleModifier::BOLD),
                )));
            }

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
