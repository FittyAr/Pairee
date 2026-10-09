use super::detail::{pane_block, row_style, text_style};
use super::{Pane, spinner_frame, wrap_text};
use crate::app::state::PluginMenuState;
use crate::config::localization::t;
use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier as StyleModifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph, Wrap},
};

/// `(label, description)` keys of the developer options 1-8; option 0
/// (active plugin) and the description of option 1 depend on the state.
const DEV_OPTIONS: [(&str, &str); 9] = [
    ("", "plugin_dev_desc_active"),
    ("plugin_dev_opt_init", "plugin_dev_desc_init"),
    ("plugin_dev_opt_lint", "plugin_dev_desc_lint"),
    ("plugin_dev_opt_package", "plugin_dev_desc_package"),
    ("plugin_dev_opt_install", "plugin_dev_desc_install"),
    ("plugin_dev_opt_submit", "plugin_dev_desc_submit"),
    // "open folder" group: dev, package and submit folders
    ("plugin_dev_opt_open_dev", "plugin_dev_desc_open_dev"),
    ("plugin_dev_opt_open_pack", "plugin_dev_desc_open_pack"),
    ("plugin_dev_opt_open_subm", "plugin_dev_desc_open_subm"),
];

/// Index of the first "open folder" option (preceded by a separator).
const OPEN_FOLDER_GROUP: usize = 6;

pub fn render_dev(
    f: &mut Frame,
    pane: &Pane,
    menu: &PluginMenuState,
    active_dev_plugin: &Option<String>,
) {
    let list_block = pane_block(pane, t("plugin_tools_title"));
    let list = List::new(option_items(pane, menu.cursor_idx, active_dev_plugin)).block(list_block);
    f.render_widget(list, pane.list_area);

    // === Right-hand console: progress bar (when loading) > results (when set) > description ===
    if menu.dev_loading {
        render_progress(f, pane, menu);
        return;
    }

    // === Idle: show previous results or the description for the current option ===
    let detail_block = pane_block(pane, t("plugin_action_console"));
    let text = if menu.dev_results.is_empty() {
        option_description(menu.cursor_idx, active_dev_plugin)
    } else {
        menu.dev_results.clone()
    };
    let text_style = text_style(pane);
    let max_width = (pane.detail_area.width as usize).saturating_sub(2);
    let detail_lines: Vec<Line> = wrap_text(&text, max_width)
        .into_iter()
        .map(|line| Line::from(Span::styled(line, text_style)))
        .collect();
    let detail_para = Paragraph::new(detail_lines)
        .block(detail_block)
        .wrap(Wrap { trim: false });
    f.render_widget(detail_para, pane.detail_area);
}

/// The option list with a separator before the "open folder" group; "new
/// plugin" is dimmed while a plugin is active.
fn option_items(
    pane: &Pane,
    cursor_idx: usize,
    active_dev_plugin: &Option<String>,
) -> Vec<ListItem<'static>> {
    // === Option 0 label: changes when a plugin is active ===
    let active_name = active_dev_plugin.as_deref().unwrap_or("");
    let opt0_label = if active_name.is_empty() {
        t("plugin_dev_opt_active_select").replace("{}", &t("plugin_dev_opt_active_none"))
    } else {
        t("plugin_dev_opt_active_change").replace("{}", active_name)
    };

    let mut list_items = Vec::new();
    for (i, (label_key, _)) in DEV_OPTIONS.iter().enumerate() {
        let is_disabled = i == 1 && active_dev_plugin.is_some();
        let style = if i == cursor_idx {
            row_style(pane, true)
        } else if is_disabled {
            Style::default().fg(Color::DarkGray)
        } else if i >= OPEN_FOLDER_GROUP {
            // Highlight "open folder" group in cyan to visually separate them.
            Style::default().fg(Color::Cyan)
        } else {
            text_style(pane)
        };
        if i == OPEN_FOLDER_GROUP {
            list_items.push(ListItem::new(Line::from(Span::styled(
                "  ───────────────────────",
                Style::default().fg(Color::DarkGray),
            ))));
        }
        let label = if i == 0 {
            opt0_label.clone()
        } else {
            t(label_key)
        };
        list_items.push(ListItem::new(Line::from(vec![Span::styled(label, style)])));
    }
    list_items
}

/// Description of the option under the cursor.
fn option_description(cursor_idx: usize, active_dev_plugin: &Option<String>) -> String {
    match DEV_OPTIONS.get(cursor_idx) {
        Some(_) if cursor_idx == 1 && active_dev_plugin.is_some() => {
            t("plugin_dev_desc_init_disabled")
        }
        Some((_, desc_key)) => t(desc_key),
        None => String::new(),
    }
}

/// Status line, gauge and the results streamed so far.
fn render_progress(f: &mut Frame, pane: &Pane, menu: &PluginMenuState) {
    let detail_area = pane.detail_area;
    let status = if menu.dev_loading_status.is_empty() {
        t("plugin_dev_progress_working")
    } else {
        menu.dev_loading_status.clone()
    };

    // Build a vertical layout: [status line][gauge][extra info if any].
    let inner_h = detail_area.height.saturating_sub(2);
    let v_chunks = if inner_h >= 4 {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // status
                Constraint::Length(3), // gauge + padding
                Constraint::Min(1),    // extra info
            ])
            .split(detail_area)
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Min(1)])
            .split(detail_area)
    };

    let status_line = Line::from(vec![Span::styled(
        format!("{} {}", spinner_frame(), status),
        Style::default().fg(Color::Yellow),
    )]);
    f.render_widget(
        Paragraph::new(status_line).style(pane.bg_style),
        v_chunks[0],
    );

    // Indeterminate progress: an empty gauge with the spinner.
    let (ratio, label) = match menu.dev_loading_progress {
        Some((cur, total)) => {
            let ratio = if total == 0 {
                0.0
            } else {
                (cur as f64 / total as f64).clamp(0.0, 1.0)
            };
            (ratio, format!("{} / {}", cur, total))
        }
        None => (0.0, spinner_frame().to_string()),
    };
    let gauge = Gauge::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(pane.border_style),
        )
        .gauge_style(
            Style::default()
                .fg(parse_color(&pane.theme.selection_bg))
                .bg(parse_color(&pane.theme.popup_bg)),
        )
        .ratio(ratio)
        .label(label);
    f.render_widget(gauge, v_chunks[1]);

    // Show any partial results that have been streamed so far.
    if !menu.dev_results.is_empty() && inner_h >= 4 {
        let dim_style = Style::default()
            .fg(parse_color(&pane.theme.popup_fg))
            .add_modifier(StyleModifier::ITALIC);
        let max_width = (v_chunks[2].width as usize).saturating_sub(2);
        let lines: Vec<Line> = wrap_text(&menu.dev_results, max_width)
            .into_iter()
            .map(|line| Line::from(Span::styled(line, dim_style)))
            .collect();
        let p = Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(pane.border_style),
            )
            .wrap(Wrap { trim: false })
            .style(pane.bg_style);
        f.render_widget(p, v_chunks[2]);
    }
}
