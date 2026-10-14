mod find;
mod line_spans;
pub mod widget;

pub use widget::{EditorView, render_editor_widget};

use super::centered_rect_in;
use crate::app::state::popup::{
    ConfirmDiscardEditorChanges, EditorConfirmOverwrite, EditorSaveAsPrompt,
};
use crate::config::localization::t;
use crate::ui::popup::kit;
use crate::ui::popup::view::{DialogView, ViewCtx};
use ratatui::{Frame, widgets::Paragraph};

impl DialogView for EditorSaveAsPrompt {
    fn render(&self, f: &mut Frame, cx: &ViewCtx<'_>) {
        let theme = cx.theme;
        kit::TextBox {
            size: (60, 7),
            title: t("editor_save_as_title"),
            border: kit::fg(theme.popup_border),
            body: kit::prompt_text(&t("editor_save_as_text"), &self.input, theme),
            body_style: theme.popup_fg(),
        }
        .with_field(kit::field_at(&t("editor_save_as_text")))
        .render(f, cx.area, theme);
    }
}

impl DialogView for EditorConfirmOverwrite {
    fn render(&self, f: &mut Frame, cx: &ViewCtx<'_>) {
        let theme = cx.theme;
        let text = t(self.reason.message_key()).replacen("{}", &self.shown.to_string_lossy(), 1);
        kit::TextBox {
            size: (60, 8),
            title: t("editor_overwrite_title"),
            border: kit::fg(theme.popup_border),
            body: text.into(),
            body_style: theme.popup_fg(),
        }
        .render_wrapped(f, cx.area, theme);
    }
}

impl DialogView for ConfirmDiscardEditorChanges {
    fn render(&self, f: &mut Frame, cx: &ViewCtx<'_>) {
        let theme = cx.theme;
        let inner = kit::frame_in(
            f,
            centered_rect_in(50, 20, cx.area),
            t("editor_discard_title"),
            kit::fg(theme.popup_border),
            theme,
        );
        let para = Paragraph::new(t("editor_discard_prompt"))
            .style(theme.popup_fg())
            .alignment(ratatui::layout::Alignment::Center);
        f.render_widget(para, inner);
    }
}
