mod brief;
mod descriptions;
mod detailed;
mod file_links;
mod file_owners;
mod full;
pub(crate) mod helpers;
mod medium;
mod wide;

use crate::app::context::AppContext;
use crate::app::state::{PanelState, PanelViewMode, SortField};
use crate::config::localization::t;
use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
};

use brief::render_brief;
use descriptions::render_descriptions;
use detailed::render_detailed;
use file_links::render_file_links;
use file_owners::render_file_owners;
use full::render_full;
use helpers::{format_file_size, get_free_space_text};
use medium::render_medium;
use wide::render_wide;

/// Entry point: dispatches to the correct view mode renderer.
/// Also renders optional footer lines (status, total info, free space) and scrollbar.
pub fn render_panel(
    f: &mut Frame,
    area: Rect,
    panel: &PanelState,
    is_active: bool,
    context: &AppContext,
) {
    let theme = &context.config.theme;
    let settings = &context.config.settings;

    let border_color = if is_active {
        parse_color(&theme.panel_border)
    } else {
        parse_color("DarkGray")
    };

    // ── Build panel title with optional sort mode letter ──────────────────────
    let mode_label = match panel.view_mode {
        PanelViewMode::Brief => t("panel_mode_brief"),
        PanelViewMode::Medium => t("panel_mode_medium"),
        PanelViewMode::Full => t("panel_mode_full"),
        PanelViewMode::Wide => t("panel_mode_wide"),
        PanelViewMode::Detailed => t("panel_mode_detailed"),
        PanelViewMode::Descriptions => t("panel_mode_desc"),
        PanelViewMode::FileOwners => t("panel_mode_owners"),
        PanelViewMode::FileLinks => t("panel_mode_links"),
        PanelViewMode::AltFull => t("panel_mode_alt"),
    };

    let sort_letter = if settings.show_sort_mode_letter {
        let letter = match panel.sort_field {
            SortField::Name => "N",
            SortField::Extension => "X",
            SortField::Size => "S",
            SortField::Date => "D",
            SortField::Unsorted => "U",
        };
        let rev = if panel.sort_reverse { "▼" } else { "▲" };
        format!("|{}{}", letter, rev)
    } else {
        String::new()
    };

    let ssh_suffix = if let Some(client) = &panel.ssh_conn {
        if let Ok(c) = client.0.lock() {
            format!(" [SSH: {}@{}]", c.username, c.host)
        } else {
            " [SSH: Locked]".to_string()
        }
    } else {
        String::new()
    };

    let title = format!(
        " {}{} [{}{}] ",
        panel.current_path.to_string_lossy(),
        ssh_suffix,
        mode_label,
        sort_letter,
    );

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
    match panel.view_mode {
        PanelViewMode::Brief => render_brief(
            f,
            list_area,
            panel,
            is_active,
            context,
            block,
            highlight_files,
        ),
        PanelViewMode::Medium => render_medium(
            f,
            list_area,
            panel,
            is_active,
            context,
            block,
            highlight_files,
        ),
        PanelViewMode::Wide => render_wide(
            f,
            list_area,
            panel,
            is_active,
            context,
            block,
            highlight_files,
        ),
        PanelViewMode::Detailed => render_detailed(
            f,
            list_area,
            panel,
            is_active,
            context,
            block,
            highlight_files,
        ),
        PanelViewMode::Descriptions => render_descriptions(
            f,
            list_area,
            panel,
            is_active,
            context,
            block,
            highlight_files,
        ),
        PanelViewMode::FileOwners => render_file_owners(
            f,
            list_area,
            panel,
            is_active,
            context,
            block,
            highlight_files,
        ),
        PanelViewMode::FileLinks => render_file_links(
            f,
            list_area,
            panel,
            is_active,
            context,
            block,
            highlight_files,
        ),
        PanelViewMode::Full | PanelViewMode::AltFull => render_full(
            f,
            list_area,
            panel,
            is_active,
            context,
            block,
            highlight_files,
        ),
    }

    // ── Optional scrollbar ────────────────────────────────────────────────────
    if show_scrollbar && !panel.entries.is_empty() {
        let inner_height = list_area.height.saturating_sub(2) as usize;
        let total = panel.entries.len();
        let mut scrollbar_state = ScrollbarState::new(total.saturating_sub(inner_height))
            .position(panel.cursor_index.min(total.saturating_sub(inner_height)));
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight);
        let scrollbar_area = Rect {
            x: list_area.x + list_area.width.saturating_sub(1),
            y: list_area.y + 1,
            width: 1,
            height: list_area.height.saturating_sub(2),
        };
        f.render_stateful_widget(scrollbar, scrollbar_area, &mut scrollbar_state);
    }

    // ── Optional footer lines ─────────────────────────────────────────────────
    if let Some(footer_area) = footer_area {
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
        let tagged = panel.selected_paths.len();

        let fg = Style::default()
            .fg(parse_color(&theme.panel_fg))
            .bg(parse_color(&theme.panel_bg));

        let mut footer_lines: Vec<Line> = Vec::new();

        if show_status {
            // Status: highlighted entry name + size
            let status_text = if let Some(entry) = panel.entries.get(panel.cursor_index) {
                if entry.is_dir {
                    format!(" {} [DIR]  {} {}", entry.name, tagged, t("label_tagged"))
                } else {
                    format!(
                        " {}  {}  {} {}",
                        entry.name,
                        format_file_size(entry.size),
                        tagged,
                        t("label_tagged")
                    )
                }
            } else {
                String::new()
            };
            footer_lines.push(Line::from(Span::styled(status_text, fg)));
        }

        if show_total {
            let files_label = if total_files == 1 {
                t("label_file")
            } else {
                t("label_files")
            };
            let dirs_label = if total_dirs == 1 {
                t("label_dir")
            } else {
                t("label_dirs")
            };
            let info_text = format!(
                " {} {}  {} {}  {}",
                total_files,
                files_label,
                total_dirs,
                dirs_label,
                format_file_size(total_size),
            );
            footer_lines.push(Line::from(Span::styled(info_text, fg)));
        }

        if show_free {
            let free_text = if panel.ssh_conn.is_some() {
                "?".to_string()
            } else {
                get_free_space_text(&panel.current_path)
            };
            footer_lines.push(Line::from(Span::styled(
                format!(" {} {}", t("label_free"), free_text),
                Style::default()
                    .fg(Color::Green)
                    .bg(parse_color(&theme.panel_bg)),
            )));
        }

        if !footer_lines.is_empty() {
            let paragraph = Paragraph::new(footer_lines);
            f.render_widget(paragraph, footer_area);
        }
    }
}
