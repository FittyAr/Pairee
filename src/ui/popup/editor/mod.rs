mod line_spans;
pub mod widget;

pub use widget::{EditorView, render_editor_widget};

use super::{centered_rect_fixed, centered_rect_in};
use crate::app::state::PopupType;
use crate::app::state::popup::ConfirmDiscardEditorChanges;
use crate::config::localization::t;
use crate::ui::popup::kit;
use crate::ui::popup::view::{DialogView, ViewCtx};
use ratatui::{
    Frame,
    layout::Rect,
    style::Style,
    widgets::{Block, Borders, Clear, Paragraph},
};

pub fn render_editor_popup(
    f: &mut Frame,
    popup: &PopupType,
    theme: &crate::ui::resolved_theme::ResolvedTheme,
    size: Rect,
) -> bool {
    match popup {
        PopupType::EditorSearchPrompt(search) => {
            super::viewer::render_search(f, "editor_search_title", search, theme, size)
        }
        PopupType::EditorSaveAsPrompt { input } => {
            kit::TextBox {
                size: (60, 7),
                title: t("editor_save_as_title"),
                border: kit::fg(theme.popup_border),
                body: kit::prompt_text(&t("editor_save_as_text"), input, theme),
                body_style: theme.popup_fg(),
            }
            .with_field(kit::field_at(&t("editor_save_as_text")))
            .render(f, size, theme);
            true
        }
        PopupType::EditorConfirmOverwrite { target, reason } => {
            let area = centered_rect_fixed(60, 8, size);
            f.render_widget(Clear, area);
            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.popup_border))
                .title(t("editor_overwrite_title"))
                .style(Style::default().bg(theme.popup_bg));
            let text = t(reason.message_key()).replacen("{}", &target.to_string_lossy(), 1);
            f.render_widget(
                Paragraph::new(text)
                    .block(block)
                    .wrap(ratatui::widgets::Wrap { trim: false })
                    .style(Style::default().fg(theme.popup_fg)),
                area,
            );
            true
        }
        _ => false,
    }
}

impl DialogView for ConfirmDiscardEditorChanges {
    fn render(&self, f: &mut Frame, cx: &ViewCtx<'_>) {
        let theme = cx.theme;
        let area = centered_rect_in(50, 20, cx.area);
        f.render_widget(Clear, area);
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.popup_border))
            .title(t("editor_discard_title"))
            .style(Style::default().bg(theme.popup_bg));
        let text = ratatui::text::Line::from(vec![ratatui::text::Span::styled(
            t("editor_discard_prompt"),
            Style::default().fg(theme.popup_fg),
        )]);
        let para = Paragraph::new(text)
            .block(block)
            .alignment(ratatui::layout::Alignment::Center);
        f.render_widget(para, area);
    }
}
