use super::detail::{field_line, pane_block, render_details, row_style, text_style};
use super::{Pane, wrap_text};
use crate::config::localization::t;
use ratatui::{
    Frame,
    style::{Color, Modifier as StyleModifier, Style},
    text::{Line, Span},
    widgets::{List, ListItem},
};

/// Strips the `.pairee` extension from a plugin name for display.
fn display_name(name: &str) -> &str {
    name.strip_suffix(".pairee").unwrap_or(name)
}

/// `(name, version, description, author)` of a registry entry.
type RegistryEntry = (String, String, String, String);

/// Widths of the name, author and version columns.
struct Columns {
    name: usize,
    author: usize,
    version: usize,
}

impl Columns {
    /// Name takes the bulk of `inner_w`; author ~18 and version ~8 are fixed.
    fn for_width(inner_w: usize) -> Self {
        let (version, author) = (8, 18);
        Self {
            name: inner_w.saturating_sub(version + author + 2), // 2 separators
            author,
            version,
        }
    }

    fn row(&self, name: &str, author: &str, version: &str) -> String {
        format!(
            " {:<name_w$}  {:<auth_w$}  {:<ver_w$}",
            name,
            author,
            version,
            name_w = self.name,
            auth_w = self.author,
            ver_w = self.version,
        )
    }
}

pub fn render_search(f: &mut Frame, pane: &Pane, menu: &crate::app::state::PluginMenuState) {
    let dim_style = Style::default().fg(Color::DarkGray);
    let list_items = if menu.is_searching {
        vec![ListItem::new(Line::from(vec![Span::styled(
            t("plugin_search_searching"),
            Style::default().fg(Color::Yellow),
        )]))]
    } else if menu.registry.is_empty() {
        vec![ListItem::new(Line::from(vec![Span::styled(
            t("plugin_search_no_results"),
            dim_style,
        )]))]
    } else {
        registry_page(pane, &menu.registry, menu.cursor_idx)
    };

    // ── Hint: edit mode indicator in the block title ─────────────────────────
    let (list_title, border_style) = if menu.editing_query {
        (
            format!("{} [{}]", t("plugin_title"), t("plugin_search_typing")),
            Style::default().fg(Color::Yellow),
        )
    } else {
        (t("plugin_title"), pane.border_style)
    };
    let list_block = pane_block(pane, list_title).border_style(border_style);
    f.render_widget(List::new(list_items).block(list_block), pane.list_area);

    // ── Detail panel ─────────────────────────────────────────────────────────
    let detail_lines = match menu.registry.get(menu.cursor_idx) {
        Some(entry) => entry_details(pane, entry),
        None if menu.registry.is_empty() => {
            vec![Line::from(Span::styled(t("plugin_no_selected"), dim_style))]
        }
        None => Vec::new(),
    };
    render_details(f, pane, detail_lines);
}

/// Header, the page of entries holding the cursor and a page indicator.
fn registry_page(
    pane: &Pane,
    registry: &[RegistryEntry],
    cursor_idx: usize,
) -> Vec<ListItem<'static>> {
    let dim_style = Style::default().fg(Color::DarkGray);
    // Usable inner width (subtract 2 for borders, 1 leading space)
    let inner_w = (pane.list_area.width as usize).saturating_sub(3);
    let columns = Columns::for_width(inner_w);

    // Leave 2 rows for borders and 1 for the page indicator at the bottom.
    let page_size = (pane.list_area.height as usize).saturating_sub(3).max(1);
    let page = cursor_idx / page_size;
    let total_pages = registry.len().div_ceil(page_size);
    let slice_start = page * page_size;
    let slice_end = (slice_start + page_size).min(registry.len());

    let mut list_items = vec![ListItem::new(Line::from(vec![Span::styled(
        columns.row("Plugin", "Author", "Version"),
        dim_style.add_modifier(StyleModifier::UNDERLINED),
    )]))];
    for (i, (name, version, _, author)) in registry[slice_start..slice_end].iter().enumerate() {
        // Truncate each column to its max width
        let row = columns.row(
            &truncate(display_name(name), columns.name),
            &truncate(author, columns.author),
            &truncate(version, columns.version),
        );
        let style = row_style(pane, slice_start + i == cursor_idx);
        list_items.push(ListItem::new(Line::from(vec![Span::styled(row, style)])));
    }

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
    list_items
}

/// Name, latest version, author and wrapped description of a registry entry.
fn entry_details(pane: &Pane, entry: &RegistryEntry) -> Vec<Line<'static>> {
    let (name, version, desc, author) = entry;
    let text = text_style(pane);
    let mut lines = vec![
        field_line(pane, "plugin_detail_lbl", display_name(name).to_string()),
        field_line(pane, "plugin_detail_latest_ver", version.clone()),
        field_line(pane, "plugin_detail_author", author.clone()),
        Line::from(""),
        Line::from(Span::styled(
            t("plugin_detail_description"),
            text.add_modifier(StyleModifier::BOLD),
        )),
    ];
    let max_width = (pane.detail_area.width as usize).saturating_sub(2);
    lines.extend(
        wrap_text(desc, max_width)
            .into_iter()
            .map(|line| Line::from(Span::styled(line, text))),
    );
    lines
}

/// Truncates a string to `max` visible characters, appending `…` if needed.
fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max.saturating_sub(1)])
    }
}
