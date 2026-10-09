mod brief;
pub(crate) mod helpers;
mod list_ctx;
mod table_view;

use crate::app::context::AppContext;
use crate::app::state::PanelState;
use crate::config::localization::t;
use crate::ui::scrollbar::{self, ScrollTargetId, ScrollbarSurface, ScrollbarUiState};
use crate::ui::scrollbar::{ScrollTarget, ScrollView};
use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use brief::render_brief;
use helpers::{build_panel_title, entry_size_text, format_file_size, free_space_text};
use list_ctx::ListCtx;
use table_view::render_table;

/// Entry point: dispatches to the correct view mode renderer.
/// Also renders optional footer lines (status, total info, free space) and scrollbar.
pub fn render_panel(
    f: &mut Frame,
    area: Rect,
    panel: &PanelState,
    is_active: bool,
    context: &AppContext,
    scrollbar: Option<&ScrollbarUiState>,
    scroll_id: ScrollTargetId,
) {
    let theme = &context.config.theme;
    let settings = &context.config.settings;

    let border_color = if is_active {
        parse_color(&theme.panel_border)
    } else {
        parse_color("DarkGray")
    };

    let title = build_panel_title(panel, settings);

    // ── Count optional footer rows ────────────────────────────────────────────
    let show_status = settings.show_status_line;
    let show_total = settings.show_files_total_information;
    let show_free = settings.show_free_size;
    let show_scrollbar = settings.show_scrollbar;
    let highlight_files = settings.highlight_files;

    let footer_height = u16::from(show_status) + u16::from(show_total) + u16::from(show_free);

    // ── Split area: [block_with_list] + [footer lines] ────────────────────────
    let constraints: Vec<Constraint> = if footer_height > 0 {
        vec![Constraint::Min(3), Constraint::Length(footer_height)]
    } else {
        vec![Constraint::Percentage(100)]
    };

    let v_split = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);

    let list_area = v_split[0];
    let footer_area = if footer_height > 0 {
        Some(v_split[1])
    } else {
        None
    };

    // ── Build the panel block ─────────────────────────────────────────────────
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(title)
        .style(Style::default().bg(parse_color(&theme.panel_bg)));

    // ── Dispatch to view-specific renderer (list area only) ───────────────────
    let ctx = ListCtx {
        panel,
        is_active,
        context,
        highlight_files,
    };
    match table_view::for_mode(panel.view_mode) {
        Some(view) => render_table(f, list_area, block, &ctx, view),
        None => render_brief(f, list_area, block, &ctx),
    }

    // ── Optional scrollbar (fractional thumb via tui-scrollbar) ───────────────
    if show_scrollbar && !panel.entries.is_empty() {
        render_scrollbar(f, list_area, panel, theme, (scrollbar, scroll_id));
    }

    // ── Optional footer lines ─────────────────────────────────────────────────
    if let Some(footer_area) = footer_area {
        let footer_lines = footer_lines(panel, context);
        if !footer_lines.is_empty() {
            f.render_widget(Paragraph::new(footer_lines), footer_area);
        }
    }
}

/// Scrollbar of the entry list, inside the panel frame.
fn render_scrollbar(
    f: &mut Frame,
    list_area: Rect,
    panel: &PanelState,
    theme: &crate::config::theme::Theme,
    (hits, id): (Option<&ScrollbarUiState>, ScrollTargetId),
) {
    let inner_height = list_area.height.saturating_sub(2) as usize;
    let total = panel.entries.len();
    let offset = scrollbar::centered_scroll(panel.cursor_index, total, inner_height);
    scrollbar::render_vertical_inside_block(
        f,
        list_area,
        ScrollView {
            content_len: total,
            viewport_len: inner_height,
            offset,
        },
        theme,
        ScrollTarget {
            surface: ScrollbarSurface::Panel,
            hits,
            id,
        },
    );
}

/// Status line, totals and free space, as enabled in the settings.
fn footer_lines(panel: &PanelState, context: &AppContext) -> Vec<Line<'static>> {
    let theme = &context.config.theme;
    let settings = &context.config.settings;
    let fg = Style::default()
        .fg(parse_color(&theme.panel_fg))
        .bg(parse_color(&theme.panel_bg));

    let mut footer_lines: Vec<Line> = Vec::new();
    if settings.show_status_line {
        footer_lines.push(Line::from(Span::styled(status_text(panel), fg)));
    }
    if settings.show_files_total_information {
        footer_lines.push(Line::from(Span::styled(totals_text(panel), fg)));
    }
    if settings.show_free_size {
        let free_text = free_space_text(panel.free_space);
        footer_lines.push(Line::from(Span::styled(
            format!(" {} {}", t("label_free"), free_text),
            Style::default()
                .fg(Color::Green)
                .bg(parse_color(&theme.panel_bg)),
        )));
    }
    footer_lines
}

/// Status: highlighted entry name + size and the number of tagged entries.
fn status_text(panel: &PanelState) -> String {
    let Some(entry) = panel.entries.get(panel.cursor_index) else {
        return String::new();
    };
    let size = entry_size_text(panel, entry).unwrap_or_else(|| "[DIR]".to_string());
    format!(
        " {}  {}  {} {}",
        entry.name,
        size,
        panel.selected_paths.len(),
        t("label_tagged")
    )
}

/// Number of files and folders and the total size of the files.
fn totals_text(panel: &PanelState) -> String {
    let total_files = panel.entries.iter().filter(|e| !e.is_dir).count();
    let total_dirs = panel
        .entries
        .iter()
        .filter(|e| e.is_dir && e.name != "..")
        .count();
    let total_size: u64 = panel
        .entries
        .iter()
        .filter(|e| !e.is_dir)
        .map(|e| e.size)
        .sum();
    let files_label = t(if total_files == 1 {
        "label_file"
    } else {
        "label_files"
    });
    let dirs_label = t(if total_dirs == 1 {
        "label_dir"
    } else {
        "label_dirs"
    });
    format!(
        " {} {}  {} {}  {}",
        total_files,
        files_label,
        total_dirs,
        dirs_label,
        format_file_size(total_size),
    )
}
