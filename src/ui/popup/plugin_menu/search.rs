use super::detail::{field_line, pane_block, render_details, row_style, text_style};
use super::{Pane, wrap_text};
use crate::config::localization::t;
use crate::ui::text_width::fit_to_width;
use ratatui::{
    Frame,
    style::{Modifier as StyleModifier, Style},
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

    /// One row, each field fitted to its column's display width.
    fn row(&self, name: &str, author: &str, version: &str) -> String {
        format!(
            " {}  {}  {}",
            fit_to_width(name, self.name),
            fit_to_width(author, self.author),
            fit_to_width(version, self.version),
        )
    }
}

pub fn render_search(f: &mut Frame, pane: &Pane, menu: &crate::app::state::PluginMenuState) {
    let dim_style = Style::default().fg(pane.theme.ui.muted);
    let list_items = if menu.is_searching {
        vec![ListItem::new(Line::from(vec![Span::styled(
            t("plugin_search_searching"),
            Style::default().fg(pane.theme.ui.warning),
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
            Style::default().fg(pane.theme.ui.warning),
        )
    } else {
        (t("plugin_title"), pane.border_style)
    };
    let list_block = pane_block(pane, list_title).border_style(border_style);
    f.render_widget(List::new(list_items).block(list_block), pane.list_area);
    if !menu.is_searching {
        register_page(pane, menu.registry.len(), menu.cursor_idx);
    }

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

/// The page of the registry list that holds the cursor.
struct Page {
    page: usize,
    total_pages: usize,
    start: usize,
    end: usize,
}

impl Page {
    fn of(pane: &Pane, len: usize, cursor_idx: usize) -> Self {
        // Leave 2 rows for borders, 1 for the column titles and 1 for the
        // page indicator at the bottom.
        let size = (pane.list_area.height as usize).saturating_sub(4).max(1);
        let page = cursor_idx / size;
        let start = page * size;
        Self {
            page,
            total_pages: len.div_ceil(size),
            start,
            end: (start + size).min(len),
        }
    }
}

/// Rows of the page take clicks (the header row above them does not); the
/// page indicator turns the page.
fn register_page(pane: &Pane, len: usize, cursor_idx: usize) {
    use crate::ui::popup::kit::mouse;
    let page = Page::of(pane, len, cursor_idx);
    let inner = pane.list_area.inner(ratatui::layout::Margin::new(1, 1));
    let rows = ratatui::layout::Rect {
        y: inner.y + 1,
        height: inner.height.saturating_sub(1),
        ..inner
    };
    mouse::list_rows(rows, page.start, page.end, cursor_idx);
    let indicator_y = rows.y + (page.end - page.start) as u16;
    if page.total_pages > 1 && indicator_y < inner.bottom() {
        let indicator = ratatui::layout::Rect {
            y: indicator_y,
            height: 1,
            ..inner
        };
        let key = if page.page + 1 < page.total_pages {
            crossterm::event::KeyCode::PageDown
        } else {
            crossterm::event::KeyCode::PageUp
        };
        mouse::keys(indicator, mouse::key(key));
    }
}

/// Header, the page of entries holding the cursor and a page indicator.
fn registry_page(
    pane: &Pane,
    registry: &[RegistryEntry],
    cursor_idx: usize,
) -> Vec<ListItem<'static>> {
    let dim_style = Style::default().fg(pane.theme.ui.muted);
    // Usable inner width (subtract 2 for borders, 1 leading space)
    let inner_w = (pane.list_area.width as usize).saturating_sub(3);
    let columns = Columns::for_width(inner_w);

    let Page {
        page,
        total_pages,
        start: slice_start,
        end: slice_end,
        ..
    } = Page::of(pane, registry.len(), cursor_idx);

    let mut list_items = vec![ListItem::new(Line::from(vec![Span::styled(
        columns.row(&t("col_plugin"), &t("col_author"), &t("about_version")),
        dim_style.add_modifier(StyleModifier::UNDERLINED),
    )]))];
    for (i, (name, version, _, author)) in registry[slice_start..slice_end].iter().enumerate() {
        let row = columns.row(display_name(name), author, version);
        let style = row_style(pane, slice_start + i == cursor_idx);
        list_items.push(ListItem::new(Line::from(vec![Span::styled(row, style)])));
    }

    if total_pages > 1 {
        let indicator = format!(
            " {:>w$}",
            t("plugin_page_indicator")
                .replacen("{}", &(page + 1).to_string(), 1)
                .replacen("{}", &total_pages.to_string(), 1),
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

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::layout::Rect;

    #[test]
    fn non_ascii_registry_fields_are_cut_by_columns() {
        let theme = crate::ui::resolved_theme::ResolvedTheme::default();
        let pane = Pane {
            list_area: Rect::new(0, 0, 50, 10),
            detail_area: Rect::new(50, 0, 30, 10),
            theme: &theme,
            border_style: Style::default(),
            bg_style: Style::default(),
        };
        let long = "Ñandú Pérez-Gutiérrez de la Peña 日本語";
        let entry = (
            format!("{long}.pairee"),
            "ü1.0.0-ñ".into(),
            String::new(),
            long.into(),
        );
        // Byte slicing used to panic in the middle of a multi-byte char.
        let rows = registry_page(&pane, &[entry], 0);
        assert_eq!(rows.len(), 2);
    }

    #[test]
    fn wide_registry_fields_keep_columns_aligned() {
        use crate::ui::text_width::display_width;
        let columns = Columns::for_width(60);
        let wide = columns.row("日本語のプラグイン名前がとても長い", "日本語の作者", "1.0");
        let narrow = columns.row("a", "b", "c");
        assert_eq!(display_width(&wide), display_width(&narrow));
    }
}
