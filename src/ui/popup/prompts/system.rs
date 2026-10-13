use crate::app::state::popup::{ErrorMessage, InfoMessage};
use crate::app::state::{PopupType, SelectMode};
use crate::config::localization::t;
use crate::ui::popup::kit::{self, TextBox};
use crate::ui::popup::view::{DialogView, ViewCtx};
use ratatui::{Frame, layout::Rect, style::Color};

pub fn render(
    f: &mut Frame,
    popup: &PopupType,
    theme: &crate::ui::resolved_theme::ResolvedTheme,
    size: Rect,
    _state: &crate::app::state::AppState,
) -> bool {
    let fg = theme.popup_fg();
    match popup {
        PopupType::ApplyCommandPrompt { input, targets } => {
            let template = t("prompt_apply_cmd_text").replacen("{}", &files_label(targets), 1);
            TextBox {
                size: (65, 10),
                title: t("prompt_apply_cmd_title"),
                border: kit::fg(Color::Yellow),
                body: kit::prompt_text(&template, input, theme),
                body_style: fg,
            }
            .with_field(kit::field_at(&template))
            .render(f, size, theme);
            true
        }
        PopupType::SelectGroupPrompt { mode, query } => {
            let (title, label) = match mode {
                SelectMode::Add => ("prompt_select_group_title", "prompt_select_group_pat"),
                SelectMode::Remove => ("prompt_unselect_group_title", "prompt_unselect_group_pat"),
            };
            let template = format!(
                "\n {}\n\n > {{}}\n\n {}",
                t(label),
                t("prompt_confirm_cancel_hint")
            );
            TextBox {
                size: (50, 9),
                title: t(title),
                border: kit::fg(Color::Cyan),
                body: kit::prompt_text(&template, query, theme),
                body_style: fg,
            }
            .with_field(kit::field_at(&template))
            .render(f, size, theme);
            true
        }
        _ => false,
    }
}

fn message_body(message: &str) -> ratatui::text::Text<'static> {
    format!("\n {}\n\n{}", message, t("prompt_dismiss_hint")).into()
}

impl DialogView for ErrorMessage {
    fn render(&self, f: &mut Frame, cx: &ViewCtx<'_>) {
        TextBox {
            size: (50, 8),
            title: t("prompt_error_title"),
            border: kit::fg(Color::Red),
            body: message_body(&self.0),
            body_style: kit::fg(Color::LightRed),
        }
        .render(f, cx.area, cx.theme);
    }
}

impl DialogView for InfoMessage {
    fn render(&self, f: &mut Frame, cx: &ViewCtx<'_>) {
        TextBox {
            size: (55, 9),
            title: t("prompt_info_title"),
            border: kit::fg(Color::Cyan),
            body: message_body(&self.0),
            body_style: cx.theme.popup_fg(),
        }
        .render(f, cx.area, cx.theme);
    }
}

/// "a, b, c" or "N files: a, b, c..." for the apply-command prompt.
fn files_label(targets: &[std::path::PathBuf]) -> String {
    let first = targets
        .iter()
        .take(3)
        .map(|p| crate::fs::file_name_lossy(p))
        .collect::<Vec<String>>()
        .join(", ");
    if targets.len() > 3 {
        t("prompt_apply_cmd_plur")
            .replacen("{}", &targets.len().to_string(), 1)
            .replacen("{}", &first, 1)
    } else {
        t("prompt_apply_cmd_sing").replacen("{}", &first, 1)
    }
}
