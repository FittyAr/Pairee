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

/// Renders inputs prompts (Branch create/rename, Stash save).
pub fn render_branch_create(
    f: &mut Frame,
    state: &crate::app::state::popup::GitBranchCreatePromptState,
    theme: &Theme,
    size: Rect,
) -> bool {
    let title = crate::config::localization::t("git_branch_create_title");
    let label = crate::config::localization::t("git_branch_create_prompt");
    render_prompt_box(
        f,
        theme,
        size,
        &title,
        &label,
        &state.input,
        state.cursor_idx,
    )
}

pub fn render_branch_rename(
    f: &mut Frame,
    state: &crate::app::state::popup::GitBranchRenamePromptState,
    theme: &Theme,
    size: Rect,
) -> bool {
    let title = crate::config::localization::t("git_branch_rename_title");
    let label =
        crate::config::localization::t("git_branch_rename_prompt").replace("{}", &state.old_name);
    render_prompt_box(
        f,
        theme,
        size,
        &title,
        &label,
        &state.input,
        state.cursor_idx,
    )
}

pub fn render_stash_save(
    f: &mut Frame,
    state: &crate::app::state::popup::GitStashSavePromptState,
    theme: &Theme,
    size: Rect,
) -> bool {
    let title = crate::config::localization::t("git_stash_save_title");
    let label = crate::config::localization::t("git_stash_save_prompt");
    render_prompt_box(
        f,
        theme,
        size,
        &title,
        &label,
        &state.input,
        state.cursor_idx,
    )
}

pub fn render_prompt_box(
    f: &mut Frame,
    theme: &Theme,
    size: Rect,
    title: &str,
    label: &str,
    input: &str,
    cursor_idx: usize,
) -> bool {
    let area = centered_rect(60, 25, size);
    f.render_widget(Clear, area);

    let border_style = Style::default().fg(Color::Cyan);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(Span::styled(
            format!(" {} ", title),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ))
        .style(Style::default().bg(parse_color(&theme.popup_bg)));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // Prompt text
            Constraint::Length(3), // Input box
            Constraint::Length(1), // Spacing
            Constraint::Length(1), // Buttons
        ])
        .split(inner);

    let prompt_p = Paragraph::new(label);
    f.render_widget(prompt_p, chunks[0]);

    let input_style = if cursor_idx == 0 {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(parse_color(&theme.popup_fg))
    };

    let input_block = Block::default()
        .borders(Borders::ALL)
        .border_style(if cursor_idx == 0 {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::Gray)
        });
    let input_p = Paragraph::new(input).style(input_style).block(input_block);
    f.render_widget(input_p, chunks[1]);

    let ok_style = if cursor_idx == 1 {
        Style::default()
            .bg(parse_color(&theme.selection_bg))
            .fg(parse_color(&theme.selection_fg))
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(parse_color(&theme.popup_fg))
    };

    let cancel_style = if cursor_idx == 2 {
        Style::default()
            .bg(parse_color(&theme.selection_bg))
            .fg(parse_color(&theme.selection_fg))
            .add_modifier(Modifier::BOLD)
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
