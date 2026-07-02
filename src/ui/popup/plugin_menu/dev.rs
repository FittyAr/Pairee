use super::wrap_text;
use crate::config::localization::t;
use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier as StyleModifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph, Wrap},
};

/// Returns a rotating spinner character (Unicode block) for the current
/// time. Used for indeterminate progress when no `(current, total)` is
/// available.
fn spinner_frame() -> &'static str {
    const FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() / 200)
        .unwrap_or(0);
    FRAMES[(now as usize) % FRAMES.len()]
}

pub fn render_dev(
    f: &mut Frame,
    list_area: Rect,
    detail_area: Rect,
    cursor_idx: usize,
    dev_results: &str,
    dev_loading: bool,
    dev_loading_status: &str,
    dev_loading_progress: Option<(usize, usize)>,
    theme: &crate::config::theme::Theme,
    border_style: Style,
    bg_style: Style,
    active_dev_plugin: &Option<String>,
) {
    let text_style = Style::default().fg(parse_color(&theme.popup_fg));
    let dim_style = Style::default().fg(parse_color(&theme.popup_fg)).add_modifier(
        StyleModifier::ITALIC,
    );

    // === Option 0 label: changes when a plugin is active ===
    let active_name = active_dev_plugin.as_deref().unwrap_or("");
    let opt0_label = if active_name.is_empty() {
        t("plugin_dev_opt_active_select")
            .replace("{}", &t("plugin_dev_opt_active_none"))
    } else {
        t("plugin_dev_opt_active_change").replace("{}", active_name)
    };

    // Build the full options list (0-8) with a visual separator before the
    // "move to folder" group.
    let dev_options: Vec<(String, bool)> = vec![
        (opt0_label, false), // 0
        (t("plugin_dev_opt_init"), false),      // 1
        (t("plugin_dev_opt_lint"), false),      // 2
        (t("plugin_dev_opt_package"), false),   // 3
        (t("plugin_dev_opt_install"), false),   // 4
        (t("plugin_dev_opt_submit"), false),    // 5
        (t("plugin_dev_opt_open_dev"), false),  // 6 - open dev folder
        (t("plugin_dev_opt_open_pack"), false), // 7 - open package folder
        (t("plugin_dev_opt_open_subm"), false), // 8 - open submit folder
    ];

    let mut list_items = Vec::new();
    for (i, (opt, _)) in dev_options.iter().enumerate() {
        let is_disabled = i == 1 && active_dev_plugin.is_some();
        let style = if i == cursor_idx {
            Style::default()
                .bg(parse_color(&theme.selection_bg))
                .fg(parse_color(&theme.selection_fg))
                .add_modifier(StyleModifier::BOLD)
        } else if is_disabled {
            Style::default().fg(Color::DarkGray)
        } else if i >= 6 {
            // Highlight "open folder" group in cyan to visually separate them.
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(parse_color(&theme.popup_fg))
        };
        // Insert a visual separator just before the navigation group.
        if i == 6 {
            list_items.push(ListItem::new(Line::from(Span::styled(
                "  ───────────────────────",
                Style::default().fg(Color::DarkGray),
            ))));
        }
        list_items.push(ListItem::new(Line::from(vec![Span::styled(
            opt.clone(),
            style,
        )])));
    }

    let list_block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(t("plugin_tools_title"))
        .style(bg_style);
    let list = List::new(list_items).block(list_block);
    f.render_widget(list, list_area);

    let detail_block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(t("plugin_action_console"))
        .style(bg_style);

    // === Right-hand console: progress bar (when loading) > results (when set) > description ===
    if dev_loading {
        let status = if dev_loading_status.is_empty() {
            t("plugin_dev_progress_working")
        } else {
            dev_loading_status.to_string()
        };

        // Build a vertical layout: [status line][gauge][extra info if any].
        let inner_h = detail_area.height.saturating_sub(2);
        let v_chunks = if inner_h >= 4 {
            ratatui::layout::Layout::default()
                .direction(ratatui::layout::Direction::Vertical)
                .constraints([
                    ratatui::layout::Constraint::Length(1), // status
                    ratatui::layout::Constraint::Length(3), // gauge + padding
                    ratatui::layout::Constraint::Min(1),    // extra info
                ])
                .split(detail_area)
        } else {
            ratatui::layout::Layout::default()
                .direction(ratatui::layout::Direction::Vertical)
                .constraints([
                    ratatui::layout::Constraint::Length(1),
                    ratatui::layout::Constraint::Min(1),
                ])
                .split(detail_area)
        };

        let status_line = Line::from(vec![Span::styled(
            format!("{} {}", spinner_frame(), status),
            Style::default().fg(Color::Yellow),
        )]);
        f.render_widget(
            Paragraph::new(status_line).style(bg_style),
            v_chunks[0],
        );

        let gauge_area = if inner_h >= 4 { v_chunks[1] } else { v_chunks[1] };
        if let Some((cur, total)) = dev_loading_progress {
            let ratio = if total == 0 {
                0.0
            } else {
                (cur as f64 / total as f64).clamp(0.0, 1.0)
            };
            let gauge = Gauge::default()
                .block(Block::default().borders(Borders::ALL).border_style(border_style))
                .gauge_style(
                    Style::default()
                        .fg(parse_color(&theme.selection_bg))
                        .bg(parse_color(&theme.popup_bg)),
                )
                .ratio(ratio)
                .label(format!("{} / {}", cur, total));
            f.render_widget(gauge, gauge_area);
        } else {
            // Indeterminate: render an empty gauge with the spinner
            let gauge = Gauge::default()
                .block(Block::default().borders(Borders::ALL).border_style(border_style))
                .gauge_style(
                    Style::default()
                        .fg(parse_color(&theme.selection_bg))
                        .bg(parse_color(&theme.popup_bg)),
                )
                .ratio(0.0)
                .label(spinner_frame());
            f.render_widget(gauge, gauge_area);
        }

        // Show any partial results that have been streamed so far.
        if !dev_results.is_empty() && inner_h >= 4 {
            let mut lines = Vec::new();
            let max_width = (v_chunks[2].width as usize).saturating_sub(2);
            for line in wrap_text(dev_results, max_width) {
                lines.push(Line::from(Span::styled(line, dim_style)));
            }
            let p = Paragraph::new(lines)
                .block(Block::default().borders(Borders::ALL).border_style(border_style))
                .wrap(Wrap { trim: false })
                .style(bg_style);
            f.render_widget(p, v_chunks[2]);
        }
        return;
    }

    // === Idle: show previous results or the description for the current option ===
    let mut detail_lines = Vec::new();
    let max_width = (detail_area.width as usize).saturating_sub(2);
    if !dev_results.is_empty() {
        for line in wrap_text(dev_results, max_width) {
            detail_lines.push(Line::from(Span::styled(line, text_style)));
        }
    } else {
        let desc_active = t("plugin_dev_desc_active");
        let desc_init = if active_dev_plugin.is_some() {
            t("plugin_dev_desc_init_disabled")
        } else {
            t("plugin_dev_desc_init")
        };
        let desc_lint = t("plugin_dev_desc_lint");
        let desc_package = t("plugin_dev_desc_package");
        let desc_install = t("plugin_dev_desc_install");
        let desc_submit = t("plugin_dev_desc_submit");
        let desc_open_dev = t("plugin_dev_desc_open_dev");
        let desc_open_pack = t("plugin_dev_desc_open_pack");
        let desc_open_subm = t("plugin_dev_desc_open_subm");

        let hint = match cursor_idx {
            0 => desc_active,
            1 => desc_init,
            2 => desc_lint,
            3 => desc_package,
            4 => desc_install,
            5 => desc_submit,
            6 => desc_open_dev,
            7 => desc_open_pack,
            8 => desc_open_subm,
            _ => String::new(),
        };
        for line in wrap_text(&hint, max_width) {
            detail_lines.push(Line::from(Span::styled(line, text_style)));
        }
    }

    let detail_para = Paragraph::new(detail_lines)
        .block(detail_block)
        .wrap(Wrap { trim: false });
    f.render_widget(detail_para, detail_area);
}
