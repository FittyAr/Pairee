//! Yes / No questions.

use crate::app::state::AdminOpKind;
use crate::app::state::popup::{
    ConfirmClearHistory, ConfirmInterrupt, ConfirmQuit, ConfirmReload, ConfirmRetryAsAdmin,
    ConfirmUndo, ConfirmUninstallPlugin, SaveSetupConfirm,
};
use crate::config::localization::t;
use crate::ui::popup::kit;
use crate::ui::popup::view::{DialogView, ViewCtx};
use ratatui::{Frame, style::Color};

/// A question box of `size` with a `title_key` title and `border` color.
fn question(
    f: &mut Frame,
    cx: &ViewCtx<'_>,
    size: (u16, u16),
    title_key: &str,
    border: Color,
    text: String,
) {
    kit::TextBox {
        size,
        title: t(title_key),
        border: kit::fg(border),
        body: text.into(),
        body_style: cx.theme.popup_fg(),
    }
    .render(f, cx.area, cx.theme);
}

impl DialogView for ConfirmQuit {
    fn render(&self, f: &mut Frame, cx: &ViewCtx<'_>) {
        let text = t("prompt_exit_text");
        question(f, cx, (45, 7), "prompt_exit_title", Color::Yellow, text);
    }
}

impl DialogView for ConfirmInterrupt {
    fn render(&self, f: &mut Frame, cx: &ViewCtx<'_>) {
        let text = t("prompt_abort_text");
        question(f, cx, (45, 7), "prompt_abort_title", Color::Red, text);
    }
}

impl DialogView for ConfirmReload {
    fn render(&self, f: &mut Frame, cx: &ViewCtx<'_>) {
        let text = t("prompt_reload_text");
        question(f, cx, (50, 8), "prompt_reload_title", Color::Yellow, text);
    }
}

impl DialogView for SaveSetupConfirm {
    fn render(&self, f: &mut Frame, cx: &ViewCtx<'_>) {
        let text = t("prompt_save_setup_text");
        question(
            f,
            cx,
            (45, 7),
            "prompt_save_setup_title",
            Color::Green,
            text,
        );
    }
}

impl DialogView for ConfirmClearHistory {
    fn render(&self, f: &mut Frame, cx: &ViewCtx<'_>) {
        let history_type = &self.history_type;
        let translated = match history_type.as_str() {
            "command" => t("history_type_command"),
            "view" => t("history_type_view"),
            "folder" => t("history_type_folder"),
            _ => history_type.clone(),
        };
        let text = t("prompt_clear_history_text").replacen("{}", &translated, 1);
        question(
            f,
            cx,
            (45, 7),
            "prompt_clear_history_title",
            Color::Yellow,
            text,
        );
    }
}

impl DialogView for ConfirmUninstallPlugin {
    fn render(&self, f: &mut Frame, cx: &ViewCtx<'_>) {
        let text = t("prompt_uninstall_plugin_text").replacen("{}", &self.name, 1);
        question(
            f,
            cx,
            (50, 7),
            "prompt_uninstall_plugin_title",
            Color::Red,
            text,
        );
    }
}

impl DialogView for ConfirmRetryAsAdmin {
    fn render(&self, f: &mut Frame, cx: &ViewCtx<'_>) {
        let text_key = match self.op_kind {
            AdminOpKind::MkDir => "prompt_sudo_mkdir_text",
            AdminOpKind::Rename { .. } => "prompt_sudo_rename_text",
        };
        kit::TextBox {
            size: (65, 8),
            title: t("prompt_sudo_title"),
            border: kit::fg(Color::Yellow),
            body: t(text_key).into(),
            body_style: cx.theme.popup_fg(),
        }
        .render_wrapped(f, cx.area, cx.theme);
    }
}

/// Undo/redo confirmation: question, entries and the Enter/Esc hint.
impl DialogView for ConfirmUndo {
    fn render(&self, f: &mut Frame, cx: &ViewCtx<'_>) {
        use crate::fs::journal::Direction;
        let title = match self.direction {
            Direction::Undo => t("journal_undo_title"),
            Direction::Redo => t("journal_redo_title"),
        };
        let mut body: Vec<ratatui::text::Line> = self
            .lines
            .iter()
            .map(|line| ratatui::text::Line::from(format!(" {line}")))
            .collect();
        body.push(ratatui::text::Line::default());
        body.push(ratatui::text::Line::from(format!(
            " {}",
            t("journal_confirm_hint")
        )));
        let height = u16::try_from(body.len() + 2).unwrap_or(u16::MAX);
        kit::TextBox {
            size: (70, height),
            title,
            border: kit::fg(Color::Yellow),
            body: body.into(),
            body_style: cx.theme.popup_fg(),
        }
        .render(f, cx.area, cx.theme);
    }
}
