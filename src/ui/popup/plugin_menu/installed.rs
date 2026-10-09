use super::detail::{field_line, pane_block, render_details, row_style, text_style};
use super::{Pane, spinner_frame};
use crate::app::state::PluginMenuState;
use crate::config::localization::t;
use crate::plugin::installed::InstalledPlugin;
use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    style::{Color, Modifier as StyleModifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, List, ListItem},
};

pub fn render_installed(f: &mut Frame, pane: &Pane, menu: &PluginMenuState) {
    let installed = menu.installed.as_slice();
    let loading = menu.installed_loading && installed.is_empty();

    let list_items = if loading {
        vec![ListItem::new(Line::from(vec![Span::styled(
            format!("  {} {}", spinner_frame(), menu.installed_loading_status),
            Style::default().fg(Color::Yellow),
        )]))]
    } else {
        installed
            .iter()
            .enumerate()
            .map(|(i, plugin)| plugin_item(pane, plugin, i == menu.cursor_idx))
            .collect()
    };
    let list = List::new(list_items).block(pane_block(pane, t("plugin_title")));
    f.render_widget(list, pane.list_area);

    if loading {
        render_loading(f, pane, &menu.installed_loading_status);
        return;
    }
    let detail_lines = match installed.get(menu.cursor_idx) {
        Some(plugin) => plugin_details(pane, plugin),
        None if installed.is_empty() => vec![Line::from(Span::styled(
            t("plugin_no_selected"),
            Style::default().fg(Color::DarkGray),
        ))],
        None => Vec::new(),
    };
    render_details(f, pane, detail_lines);
}

/// `name vX [P][T|U][▲]` row: pinned, trusted/untrusted, update available.
fn plugin_item(pane: &Pane, plugin: &InstalledPlugin, selected: bool) -> ListItem<'static> {
    let pin_badge = if plugin.pinned { " [P]" } else { "" };
    let trust_badge = if plugin.trusted { " [T]" } else { " [U]" };
    let update_badge = if plugin.update_available.is_some() {
        " [▲]"
    } else {
        ""
    };
    ListItem::new(Line::from(vec![Span::styled(
        format!(
            "  {} v{}{}{}{}",
            plugin.name, plugin.version, pin_badge, trust_badge, update_badge
        ),
        row_style(pane, selected),
    )]))
}

/// Indeterminate gauge and status line while the plugin list loads.
fn render_loading(f: &mut Frame, pane: &Pane, loading_status: &str) {
    let status = if loading_status.is_empty() {
        t("plugin_dev_progress_loading_index")
    } else {
        loading_status.to_string()
    };
    let gauge = Gauge::default()
        .block(Block::default().borders(Borders::NONE))
        .gauge_style(
            Style::default()
                .fg(parse_color(&pane.theme.selection_bg))
                .bg(parse_color(&pane.theme.popup_bg)),
        )
        .ratio(0.0)
        .label("");
    f.render_widget(gauge, pane.detail_area);
    let status_line = Line::from(Span::styled(
        format!("{} {}", spinner_frame(), status),
        Style::default().fg(Color::Yellow),
    ));
    render_details(f, pane, vec![status_line]);
}

/// Name, version, trust, pin, commands and update state of `plugin`.
fn plugin_details(pane: &Pane, plugin: &InstalledPlugin) -> Vec<Line<'static>> {
    let pick = |flag: bool, yes: &str, no: &str| t(if flag { yes } else { no });
    let commands = if plugin.commands.is_empty() {
        t("plugin_detail_commands_none")
    } else {
        plugin.commands.join(", ")
    };
    let mut lines = vec![
        field_line(pane, "plugin_detail_name", plugin.name.clone()),
        field_line(pane, "plugin_detail_version", plugin.version.clone()),
        field_line(
            pane,
            "plugin_detail_trust",
            pick(
                plugin.trusted,
                "plugin_detail_trusted_desc",
                "plugin_detail_untrusted_desc",
            ),
        ),
        field_line(
            pane,
            "plugin_detail_pinned",
            pick(
                plugin.pinned,
                "plugin_detail_pinned_yes",
                "plugin_detail_pinned_no",
            ),
        ),
        field_line(pane, "plugin_detail_commands", commands),
    ];
    lines.push(match &plugin.update_available {
        Some(new_ver) => {
            let text = text_style(pane);
            Line::from(vec![
                Span::styled(
                    t("plugin_detail_update_avail"),
                    text.add_modifier(StyleModifier::BOLD).fg(Color::Yellow),
                ),
                Span::styled(
                    format!("v{}{}", new_ver, t("plugin_detail_press_update")),
                    text.fg(Color::Yellow),
                ),
            ])
        }
        None => field_line(
            pane,
            "plugin_detail_update_status",
            t("plugin_detail_up_to_date"),
        ),
    });
    lines
}
