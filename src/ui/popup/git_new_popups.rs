use crate::app::state::PopupType;
use crate::config::theme::Theme;
use crate::ui::popup::centered_rect;
use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

/// Main entry point to render the new Git popups.
pub fn render(f: &mut Frame, popup: &PopupType, theme: &Theme, size: Rect) -> bool {
    match popup {
        PopupType::GitDiffView { .. } => render_diff_view(f, popup, theme, size),
        PopupType::GitBranchCreatePrompt { .. }
        | PopupType::GitBranchRenamePrompt { .. }
        | PopupType::GitStashSavePrompt { .. } => render_git_prompt(f, popup, theme, size),
        PopupType::GitConfirmAction { .. } => render_confirm_action(f, popup, theme, size),
        _ => false,
    }
}

/// Renders the unified diff viewer.
fn render_diff_view(f: &mut Frame, popup: &PopupType, theme: &Theme, size: Rect) -> bool {
    if let PopupType::GitDiffView {
        file_path,
        commit_hash,
        diff_content,
        scroll_y,
        ..
    } = popup
    {
        let area = centered_rect(80, 80, size);
        f.render_widget(Clear, area);

        let border_style = Style::default().fg(Color::Cyan);
        let title = if let Some(path) = file_path {
            crate::config::localization::t("git_diff_view_title").replace("{}", path)
        } else if let Some(hash) = commit_hash {
            crate::config::localization::t("git_diff_commit_title").replace("{}", hash)
        } else {
            " Git Diff ".to_string()
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(Span::styled(
                title,
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ))
            .style(Style::default().bg(parse_color(&theme.popup_bg)));

        let inner = block.inner(area);
        f.render_widget(block, area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(3),
                Constraint::Length(1), // hint
            ])
            .split(inner);

        let content_area = chunks[0];
        let hint_area = chunks[1];

        // Process lines and colors
        let height = content_area.height as usize;
        let lines: Vec<Line> = diff_content
            .lines()
            .skip(*scroll_y)
            .take(height)
            .map(|line| {
                let style = if line.starts_with('+') && !line.starts_with("+++") {
                    Style::default().fg(Color::Green)
                } else if line.starts_with('-') && !line.starts_with("---") {
                    Style::default().fg(Color::Red)
                } else if line.starts_with("@@") {
                    Style::default().fg(Color::Cyan)
                } else {
                    Style::default().fg(parse_color(&theme.popup_fg))
                };
                Line::from(Span::styled(line.to_string(), style))
            })
            .collect();

        f.render_widget(Paragraph::new(lines), content_area);

        // Hint bar
        let hint = crate::config::localization::t("git_diff_view_hint");
        f.render_widget(
            Paragraph::new(Span::styled(hint, Style::default().fg(Color::Yellow))),
            hint_area,
        );

        true
    } else {
        false
    }
}

/// Renders inputs prompts (Branch create/rename, Stash save).
fn render_git_prompt(f: &mut Frame, popup: &PopupType, theme: &Theme, size: Rect) -> bool {
    let (title, label, input, cursor_idx) = match popup {
        PopupType::GitBranchCreatePrompt { input, cursor_idx, .. } => (
            crate::config::localization::t("git_branch_create_title"),
            crate::config::localization::t("git_branch_create_prompt"),
            input,
            cursor_idx,
        ),
        PopupType::GitBranchRenamePrompt { input, cursor_idx, old_name, .. } => (
            crate::config::localization::t("git_branch_rename_title"),
            crate::config::localization::t("git_branch_rename_prompt").replace("{}", old_name),
            input,
            cursor_idx,
        ),
        PopupType::GitStashSavePrompt { input, cursor_idx, .. } => (
            crate::config::localization::t("git_stash_save_title"),
            crate::config::localization::t("git_stash_save_prompt"),
            input,
            cursor_idx,
        ),
        _ => return false,
    };

    let area = centered_rect(60, 25, size);
    f.render_widget(Clear, area);

    let border_style = Style::default().fg(Color::Cyan);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(Span::styled(
            title,
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ))
        .style(Style::default().bg(parse_color(&theme.popup_bg)));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // label
            Constraint::Length(3), // input box
            Constraint::Length(1), // spacer
            Constraint::Length(1), // buttons
        ])
        .split(inner);

    // 1. Label
    f.render_widget(Paragraph::new(label), chunks[0]);

    // 2. Input Box
    let is_input_focused = *cursor_idx == 0;
    let input_border_color = if is_input_focused { Color::Yellow } else { Color::DarkGray };
    let input_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(input_border_color));
    
    let input_para = Paragraph::new(input.as_str()).block(input_block);
    f.render_widget(input_para, chunks[1]);

    // 3. Buttons
    let ok_style = if *cursor_idx == 1 {
        Style::default().bg(parse_color(&theme.selection_bg)).fg(parse_color(&theme.selection_fg)).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(parse_color(&theme.popup_fg))
    };
    let cancel_style = if *cursor_idx == 2 {
        Style::default().bg(parse_color(&theme.selection_bg)).fg(parse_color(&theme.selection_fg)).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(parse_color(&theme.popup_fg))
    };

    let buttons_line = Line::from(vec![
        Span::styled(" [ OK ] ", ok_style),
        Span::raw("    "),
        Span::styled(" [ Cancel ] ", cancel_style),
    ]);
    
    let buttons_para = Paragraph::new(buttons_line).alignment(ratatui::layout::Alignment::Center);
    f.render_widget(buttons_para, chunks[3]);

    true
}

/// Renders a generic confirm action popup.
fn render_confirm_action(f: &mut Frame, popup: &PopupType, theme: &Theme, size: Rect) -> bool {
    if let PopupType::GitConfirmAction { message, .. } = popup {
        // Centered small confirmation box
        let area = centered_rect(50, 20, size);
        f.render_widget(Clear, area);

        let border_style = Style::default().fg(Color::Cyan);
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(Span::styled(
                " Confirm Git Action ",
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ))
            .style(Style::default().bg(parse_color(&theme.popup_bg)));

        let inner = block.inner(area);
        f.render_widget(block, area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(2),    // Message
                Constraint::Length(1), // Spacer
                Constraint::Length(1), // Buttons: Yes / No
            ])
            .split(inner);

        // Render message
        let msg_para = Paragraph::new(message.as_str())
            .alignment(ratatui::layout::Alignment::Center)
            .style(Style::default().fg(parse_color(&theme.popup_fg)));
        f.render_widget(msg_para, chunks[0]);

        // Yes/No Buttons. By default YES is focused (we handle Enter / Esc).
        let yes_style = Style::default()
            .bg(parse_color(&theme.selection_bg))
            .fg(parse_color(&theme.selection_fg))
            .add_modifier(Modifier::BOLD);
        let no_style = Style::default().fg(parse_color(&theme.popup_fg));

        let buttons_line = Line::from(vec![
            Span::styled(" [ Yes (Enter) ] ", yes_style),
            Span::raw("    "),
            Span::styled(" [ No (Esc) ] ", no_style),
        ]);
        let buttons_para = Paragraph::new(buttons_line).alignment(ratatui::layout::Alignment::Center);
        f.render_widget(buttons_para, chunks[2]);

        true
    } else {
        false
    }
}
