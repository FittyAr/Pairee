use crate::app::state::popup::ConfirmDelete;
use crate::config::localization::t;
use crate::ui::popup::centered_rect_fixed;
use crate::ui::popup::view::{DialogView, ViewCtx};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Clear, Paragraph},
};

impl DialogView for ConfirmDelete {
    fn render(&self, f: &mut Frame, cx: &ViewCtx<'_>) {
        let (theme, size) = (cx.theme, cx.area);
        let ConfirmDelete {
            paths,
            cursor_idx,
            to_trash,
            folders,
        } = self;
        let area = centered_rect_fixed(50, 8, size);
        f.render_widget(Clear, area);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Red))
            .title(t(if *to_trash {
                "prompt_trash_title"
            } else {
                "prompt_delete_title"
            }))
            .style(Style::default().bg(theme.popup_bg));

        let inner = block.inner(area);
        f.render_widget(block, area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(4), Constraint::Length(2)])
            .split(inner);

        let folders_count = *folders;
        let files_count = paths.len().saturating_sub(folders_count);

        let part_files = if files_count > 0 {
            Some(t("delete_confirm_files").replacen("{}", &files_count.to_string(), 1))
        } else {
            None
        };

        let part_folders = if folders_count > 0 {
            Some(t("delete_confirm_folders").replacen("{}", &folders_count.to_string(), 1))
        } else {
            None
        };

        let target_desc = match (part_files, part_folders) {
            (Some(f), Some(d)) => format!("{}{}{}", f, t("delete_confirm_conjunction"), d),
            (Some(f), None) => f,
            (None, Some(d)) => d,
            (None, None) => "0 items".to_string(),
        };

        let text = t("delete_confirm_msg").replacen("{}", &target_desc, 1);

        f.render_widget(
            Paragraph::new(text)
                .alignment(ratatui::layout::Alignment::Center)
                .wrap(ratatui::widgets::Wrap { trim: true })
                .style(Style::default().fg(theme.popup_fg)),
            chunks[0],
        );

        render_buttons(f, chunks[1], *cursor_idx, theme);
    }
}

/// `[Delete]  [Cancel]`, the focused one highlighted; Left + Enter deletes,
/// Esc cancels.
fn render_buttons(
    f: &mut Frame,
    area: Rect,
    cursor_idx: usize,
    theme: &crate::ui::resolved_theme::ResolvedTheme,
) {
    let active_style = Style::default().bg(Color::Cyan).fg(Color::Black);
    let normal_style = Style::default().fg(theme.popup_fg);
    let btn1 = if cursor_idx == 0 {
        active_style
    } else {
        normal_style
    };
    let btn2 = if cursor_idx == 1 {
        active_style
    } else {
        normal_style
    };
    let labels = [t("btn_delete_bracket"), t("btn_cancel_bracket")];
    let btns = ratatui::text::Line::from(vec![
        ratatui::text::Span::styled(labels[0].clone(), btn1),
        ratatui::text::Span::raw("  "),
        ratatui::text::Span::styled(labels[1].clone(), btn2),
    ]);
    {
        use crate::ui::popup::kit::mouse::{key, span_buttons};
        use crossterm::event::{KeyCode, KeyEvent};
        // Left focuses [Delete], Enter runs the focused button.
        let delete = vec![
            KeyEvent::from(KeyCode::Left),
            KeyEvent::from(KeyCode::Enter),
        ];
        span_buttons(area, &labels, (2, true), vec![delete, key(KeyCode::Esc)]);
    }
    f.render_widget(
        Paragraph::new(btns).alignment(ratatui::layout::Alignment::Center),
        area,
    );
}

#[cfg(test)]
mod tests {
    use crate::config::localization::t;
    use crate::test_harness::Harness;

    #[test]
    fn folders_are_counted_once_from_the_listing() {
        let mut h = Harness::new();
        h.mkdir("work/left/sub");
        h.reread().focus("sub").keys("@delete");
        let folders = t("delete_confirm_folders").replacen("{}", "1", 1);
        let shown: String = folders.chars().take(11).collect();
        h.assert_screen(&shown);
        // Redrawing does not look at the disk again.
        std::fs::remove_dir(h.left().join("sub")).unwrap();
        h.render();
        h.assert_screen(&shown);
    }
}
