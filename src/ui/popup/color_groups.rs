use super::centered_rect;
use crate::app::state::PopupType;
use crate::config::theme::Theme;
use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

pub const THEME_PROPS: [&str; 16] = [
    "panel_bg",
    "panel_fg",
    "panel_border",
    "selection_bg",
    "selection_fg",
    "marked_fg",
    "header_bg",
    "header_fg",
    "cli_bg",
    "cli_fg",
    "fkey_num_fg",
    "fkey_text_fg",
    "fkey_bg",
    "popup_bg",
    "popup_fg",
    "popup_border",
];

pub fn get_theme_prop(theme: &Theme, idx: usize) -> &String {
    match idx {
        0 => &theme.panel_bg,
        1 => &theme.panel_fg,
        2 => &theme.panel_border,
        3 => &theme.selection_bg,
        4 => &theme.selection_fg,
        5 => &theme.marked_fg,
        6 => &theme.header_bg,
        7 => &theme.header_fg,
        8 => &theme.cli_bg,
        9 => &theme.cli_fg,
        10 => &theme.fkey_num_fg,
        11 => &theme.fkey_text_fg,
        12 => &theme.fkey_bg,
        13 => &theme.popup_bg,
        14 => &theme.popup_fg,
        15 => &theme.popup_border,
        _ => &theme.panel_bg,
    }
}

pub fn render_color_groups_popup(
    f: &mut Frame,
    popup: &PopupType,
    theme: &Theme,
    size: Rect,
) -> bool {
    if let PopupType::ColorGroupsDialog {
        cursor_idx,
        editing,
        edit_buffer,
        theme: edit_theme,
    } = popup
    {
        let area = centered_rect(60, 60, size);
        f.render_widget(Clear, area);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(parse_color(&theme.popup_border)))
            .title(" Color Groups ")
            .style(Style::default().bg(parse_color(&theme.popup_bg)));

        let inner = block.inner(area);
        f.render_widget(block, area);

        let mut lines = Vec::new();
        let scroll_start = cursor_idx.saturating_sub(inner.height as usize / 2);

        for (i, prop_name) in THEME_PROPS
            .iter()
            .enumerate()
            .skip(scroll_start)
            .take(inner.height as usize)
        {
            let is_cursor = i == *cursor_idx;

            let prop_value = if is_cursor && *editing {
                format!("{}_", edit_buffer)
            } else {
                get_theme_prop(edit_theme, i).clone()
            };

            let style = if is_cursor {
                Style::default()
                    .bg(parse_color(&theme.selection_bg))
                    .fg(parse_color(&theme.selection_fg))
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(parse_color(&theme.popup_fg))
            };

            let line_text = format!(" {:<20} < {:^15} >", prop_name, prop_value);
            lines.push(Line::from(Span::styled(line_text, style)));
        }

        f.render_widget(Paragraph::new(lines), inner);
        return true;
    }
    false
}
