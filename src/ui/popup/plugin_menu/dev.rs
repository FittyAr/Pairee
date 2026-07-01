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

pub fn render_dev(
    f: &mut Frame,
    list_area: Rect,
    detail_area: Rect,
    cursor_idx: usize,
    dev_results: &str,
    theme: &crate::config::theme::Theme,
    border_style: Style,
    bg_style: Style,
    active_dev_plugin: &Option<String>,
) {
    let text_style = Style::default().fg(parse_color(&theme.popup_fg));

    let active_name = active_dev_plugin.as_deref().unwrap_or("");
    let active_label = if active_name.is_empty() {
        t("plugin_dev_opt_active").replace("{}", &t("plugin_dev_opt_active_none"))
    } else {
        t("plugin_dev_opt_active").replace("{}", active_name)
    };

    let dev_options = [
        active_label,
        t("plugin_dev_opt_init"),
        t("plugin_dev_opt_lint"),
        t("plugin_dev_opt_package"),
        t("plugin_dev_opt_install"),
        t("plugin_dev_opt_submit"),
    ];

    let mut list_items = Vec::new();
    for (i, opt) in dev_options.iter().enumerate() {
        let style = if i == cursor_idx {
            Style::default()
                .bg(parse_color(&theme.selection_bg))
                .fg(parse_color(&theme.selection_fg))
                .add_modifier(StyleModifier::BOLD)
        } else if i == 1 && active_dev_plugin.is_some() {
            Style::default().fg(Color::DarkGray)
        } else {
            Style::default().fg(parse_color(&theme.popup_fg))
        };
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

        let hint = match cursor_idx {
            0 => desc_active,
            1 => desc_init,
            2 => desc_lint,
            3 => desc_package,
            4 => desc_install,
            5 => desc_submit,
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
