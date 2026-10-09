use crate::app::state::PopupType;
use crate::config::localization::t;
use crate::ui::popup::kit;
use ratatui::{Frame, layout::Rect, style::Color};

pub fn render(
    f: &mut Frame,
    popup: &PopupType,
    theme: &crate::config::theme::Theme,
    size: Rect,
) -> bool {
    let (dims, title_key, border, text) = match popup {
        PopupType::ConfirmQuit => (
            (45, 7),
            "prompt_exit_title",
            Color::Yellow,
            t("prompt_exit_text"),
        ),
        PopupType::ConfirmInterrupt => (
            (45, 7),
            "prompt_abort_title",
            Color::Red,
            t("prompt_abort_text"),
        ),
        PopupType::ConfirmReload => (
            (50, 8),
            "prompt_reload_title",
            Color::Yellow,
            t("prompt_reload_text"),
        ),
        PopupType::ConfirmClearHistory { history_type } => {
            let hist_type_translated = match history_type.as_str() {
                "command" => t("history_type_command"),
                "view" => t("history_type_view"),
                "folder" => t("history_type_folder"),
                _ => history_type.clone(),
            };
            let text = t("prompt_clear_history_text").replacen("{}", &hist_type_translated, 1);
            ((45, 7), "prompt_clear_history_title", Color::Yellow, text)
        }
        PopupType::SaveSetupConfirm => (
            (45, 7),
            "prompt_save_setup_title",
            Color::Green,
            t("prompt_save_setup_text"),
        ),
        PopupType::ConfirmUndo { direction, lines } => {
            render_undo(f, *direction, lines, theme, size);
            return true;
        }
        _ => return false,
    };
    kit::TextBox {
        size: dims,
        title: t(title_key),
        border: kit::fg(border),
        body: text.into(),
        body_style: kit::popup_fg(theme),
    }
    .render(f, size, theme);
    true
}

/// Undo/redo confirmation: question, entries and the Enter/Esc hint.
fn render_undo(
    f: &mut Frame,
    direction: crate::fs::journal::Direction,
    lines: &[String],
    theme: &crate::config::theme::Theme,
    size: Rect,
) {
    use crate::fs::journal::Direction;
    use crate::ui::popup::kit::TextBox;
    let title = match direction {
        Direction::Undo => t("journal_undo_title"),
        Direction::Redo => t("journal_redo_title"),
    };
    let mut body: Vec<ratatui::text::Line> = lines
        .iter()
        .map(|line| ratatui::text::Line::from(format!(" {line}")))
        .collect();
    body.push(ratatui::text::Line::default());
    body.push(ratatui::text::Line::from(format!(
        " {}",
        t("journal_confirm_hint")
    )));
    let height = u16::try_from(body.len() + 2).unwrap_or(u16::MAX);
    TextBox {
        size: (70, height),
        title,
        border: kit::fg(Color::Yellow),
        body: body.into(),
        body_style: kit::popup_fg(theme),
    }
    .render(f, size, theme);
}
