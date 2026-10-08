use crate::app::state::{AdminOpKind, PopupType, SelectMode};
use crate::config::localization::t;
use crate::ui::popup::kit::{self, TextBox};
use ratatui::{Frame, layout::Rect, style::Color};

pub fn render(
    f: &mut Frame,
    popup: &PopupType,
    theme: &crate::config::theme::Theme,
    size: Rect,
    _state: &crate::app::state::AppState,
) -> bool {
    let fg = kit::popup_fg(theme);
    let text_box = match popup {
        PopupType::ConfirmRetryAsAdmin { op_kind, .. } => {
            let text_key = match op_kind {
                AdminOpKind::MkDir => "prompt_sudo_mkdir_text",
                AdminOpKind::Rename { .. } => "prompt_sudo_rename_text",
            };
            TextBox {
                size: (65, 8),
                title: t("prompt_sudo_title"),
                border: kit::fg(Color::Yellow),
                body: t(text_key).into(),
                body_style: fg,
            }
            .render_wrapped(f, size, theme);
            return true;
        }
        PopupType::Error(message) => TextBox {
            size: (50, 8),
            title: t("prompt_error_title"),
            border: kit::fg(Color::Red),
            body: message_body(message),
            body_style: kit::fg(Color::LightRed),
        },
        PopupType::Info(message) => TextBox {
            size: (55, 9),
            title: t("prompt_info_title"),
            border: kit::fg(Color::Cyan),
            body: message_body(message),
            body_style: fg,
        },
        PopupType::ApplyCommandPrompt { input, targets } => {
            let template = t("prompt_apply_cmd_text").replacen("{}", &files_label(targets), 1);
            TextBox {
                size: (65, 10),
                title: t("prompt_apply_cmd_title"),
                border: kit::fg(Color::Yellow),
                body: kit::prompt_text(&template, input, theme),
                body_style: fg,
            }
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
        }
        _ => return false,
    };
    text_box.render(f, size, theme);
    true
}

fn message_body(message: &str) -> ratatui::text::Text<'static> {
    format!("\n {}\n\n{}", message, t("prompt_dismiss_hint")).into()
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
