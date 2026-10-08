use super::super::centered_rect_fixed;
use crate::app::context::AppContext;
use crate::app::state::popup::SshField;
use crate::app::state::{PopupType, SshConnectPromptState};
use crate::config::localization::t;
use crate::ui::popup::kit;
use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
};

pub fn render(
    f: &mut Frame,
    popup: &PopupType,
    theme: &crate::config::theme::Theme,
    size: Rect,
    context: &AppContext,
) -> bool {
    if let PopupType::SshConnectPrompt(prompt) = popup {
        let cursor_idx = &prompt.cursor_idx;
        let selected_preset_idx = &prompt.selected_preset_idx;
        let area = centered_rect_fixed(75, 12, size);
        f.render_widget(Clear, area);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .title(t("prompt_ssh_title"))
            .style(Style::default().bg(parse_color(&theme.popup_bg)));
        let inner = block.inner(area);
        f.render_widget(block, area);

        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(8),    // Form columns
                Constraint::Length(1), // Separator
                Constraint::Length(1), // Buttons
            ])
            .split(inner);

        let form_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(25), // Presets list
                Constraint::Length(1),  // Vertical separator
                Constraint::Min(30),    // Inputs
            ])
            .split(main_chunks[0]);

        let active_style = Style::default().bg(Color::Cyan).fg(Color::Black);
        let normal_style = Style::default().fg(parse_color(&theme.popup_fg));

        // Left column: Presets List
        let presets = &context.config.settings.ssh_presets;
        let mut list_items = Vec::new();
        if presets.is_empty() {
            list_items.push(ListItem::new(ratatui::text::Line::from(vec![
                ratatui::text::Span::styled(" <No Presets> ", Style::default().fg(Color::DarkGray)),
            ])));
        } else {
            for (i, p) in presets.iter().enumerate() {
                let is_current = Some(i) == *selected_preset_idx;
                let is_active_field = *cursor_idx == 0;

                let style = if is_current && is_active_field {
                    active_style
                } else if is_current {
                    Style::default().bg(Color::DarkGray).fg(Color::White)
                } else {
                    normal_style
                };

                list_items.push(ListItem::new(ratatui::text::Line::from(vec![
                    ratatui::text::Span::styled(format!("  {}  ", p.name), style),
                ])));
            }
        }

        let presets_block = Block::default()
            .borders(Borders::NONE)
            .title(format!(" {} ", t("ssh_presets_title").trim()));
        let list = List::new(list_items)
            .block(presets_block)
            .style(Style::default().bg(parse_color(&theme.popup_bg)));
        f.render_widget(list, form_chunks[0]);

        // Vertical separator
        let sep_str_vertical = ratatui::symbols::line::VERTICAL;
        for y in form_chunks[1].y..(form_chunks[1].y + form_chunks[1].height) {
            f.render_widget(
                Paragraph::new(sep_str_vertical).style(Style::default().fg(Color::Cyan)),
                Rect::new(form_chunks[1].x, y, 1, 1),
            );
        }

        // Right column: Inputs
        let input_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Spacer / Title
                Constraint::Length(1), // Name
                Constraint::Length(1), // Host
                Constraint::Length(1), // Port
                Constraint::Length(1), // Username
                Constraint::Length(1), // Password
                Constraint::Length(1), // Key Path
            ])
            .split(form_chunks[2]);

        f.render_widget(
            Paragraph::new(format!(" {}", t("ssh_details_title")))
                .style(Style::default().fg(Color::Yellow)),
            input_chunks[0],
        );
        let styles = kit::FocusStyles {
            active: active_style,
            normal: normal_style,
        };
        for (field, chunk) in SshField::ALL.iter().zip(&input_chunks[1..]) {
            f.render_widget(field_line(prompt, *field, styles), *chunk);
        }

        // Bottom horizontal separator
        let sep_str_horizontal = ratatui::symbols::line::HORIZONTAL.repeat(inner.width as usize);
        f.render_widget(
            Paragraph::new(sep_str_horizontal).style(Style::default().fg(Color::Cyan)),
            main_chunks[1],
        );

        // Buttons at the bottom
        let buttons = [
            "btn_connect_braced",
            "btn_save_preset",
            "btn_delete_preset",
            "btn_cancel_bracket",
        ]
        .map(|key| format!(" {} ", t(key)));
        f.render_widget(
            kit::button_bar(
                &buttons,
                cursor_idx.checked_sub(SshConnectPromptState::BUTTON_CONNECT),
                styles,
            ),
            main_chunks[2],
        );

        true
    } else {
        false
    }
}

/// `" Label:        value"` for one field (password masked), with a cursor
/// when focused.
fn field_line(
    prompt: &SshConnectPromptState,
    field: SshField,
    styles: kit::FocusStyles,
) -> Paragraph<'static> {
    let focused = prompt.cursor_idx == field.row();
    let style = styles.pick(focused);
    let mut label = t(field.label_key()).trim().to_string();
    if !label.ends_with(':') {
        label.push(':');
    }
    let mut spans = vec![ratatui::text::Span::styled(
        format!(" {:<14} ", label),
        style,
    )];
    let value = &prompt.fields[field as usize];
    if field == SshField::Password {
        let masked = "*".repeat(value.text().chars().count());
        let cursor = if focused { "_" } else { "" };
        spans.push(ratatui::text::Span::styled(
            format!("{masked}{cursor}"),
            style,
        ));
    } else {
        spans.extend(kit::field_spans(value, style, styles.cursor(), focused));
    }
    Paragraph::new(ratatui::text::Line::from(spans))
}
