use crate::app::state::SelectMode;
use crate::app::state::popup::{ApplyCommandPrompt, ErrorMessage, InfoMessage, SelectGroupPrompt};
use crate::config::localization::t;
use crate::ui::popup::kit::{self, TextBox};
use crate::ui::popup::view::{DialogView, ViewCtx};
use crate::ui::text_width::display_width;
use crate::ui::wrap::wrap_text;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::{Line, Text};

impl DialogView for ApplyCommandPrompt {
    fn render(&self, f: &mut Frame, cx: &ViewCtx<'_>) {
        let template = t("prompt_apply_cmd_text").replacen("{}", &files_label(&self.targets), 1);
        kit::PromptBox {
            size: (65, 10),
            title: t("prompt_apply_cmd_title"),
            border: kit::fg(cx.theme.ui.warning),
            template: &template,
            field: &self.input,
        }
        .render(f, cx.area, cx.theme);
    }
}

impl DialogView for SelectGroupPrompt {
    fn render(&self, f: &mut Frame, cx: &ViewCtx<'_>) {
        let (title, label) = match self.mode {
            SelectMode::Add => ("prompt_select_group_title", "prompt_select_group_pat"),
            SelectMode::Remove => ("prompt_unselect_group_title", "prompt_unselect_group_pat"),
        };
        let template = format!(
            "\n {}\n\n > {{}}\n\n {}",
            t(label),
            t("prompt_confirm_cancel_hint")
        );
        kit::PromptBox {
            size: (50, 9),
            title: t(title),
            border: kit::fg(cx.theme.ui.accent),
            template: &template,
            field: &self.query,
        }
        .render(f, cx.area, cx.theme);
    }
}

/// Widest a message box grows before its text wraps.
const MESSAGE_MAX_WIDTH: u16 = 76;

/// Size and body of a message box: at least `min`, wider (up to
/// [`MESSAGE_MAX_WIDTH`]) and taller as the message needs, within
/// `screen`, so the whole message shows word-wrapped above the dismiss
/// hint.
fn message_box(message: &str, min: (u16, u16), screen: Rect) -> ((u16, u16), Text<'static>) {
    // Borders plus a column of margin on each side.
    const FRAME: usize = 4;
    let widest = message.lines().map(display_width).max().unwrap_or(0);
    let width = u16::try_from(widest + FRAME)
        .unwrap_or(u16::MAX)
        .clamp(min.0, MESSAGE_MAX_WIDTH.max(min.0))
        .min(screen.width);
    let inner = usize::from(width).saturating_sub(FRAME);
    let mut lines = vec![Line::default()];
    lines.extend(
        wrap_text(message, inner)
            .into_iter()
            .map(|l| Line::from(format!(" {l}"))),
    );
    lines.push(Line::default());
    lines.extend(
        wrap_text(&t("prompt_dismiss_hint"), inner + 2)
            .into_iter()
            .map(Line::from),
    );
    let height = u16::try_from(lines.len() + 2)
        .unwrap_or(u16::MAX)
        .max(min.1);
    ((width, height), Text::from(lines))
}

impl DialogView for ErrorMessage {
    fn render(&self, f: &mut Frame, cx: &ViewCtx<'_>) {
        let (size, body) = message_box(&self.0, (50, 8), cx.area);
        TextBox {
            size,
            title: t("prompt_error_title"),
            border: kit::fg(cx.theme.ui.danger),
            body,
            body_style: kit::fg(cx.theme.ui.error),
        }
        .render(f, cx.area, cx.theme);
    }
}

impl DialogView for InfoMessage {
    fn render(&self, f: &mut Frame, cx: &ViewCtx<'_>) {
        let (size, body) = message_box(&self.0, (55, 9), cx.area);
        TextBox {
            size,
            title: t("prompt_info_title"),
            border: kit::fg(cx.theme.ui.accent),
            body,
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

#[cfg(test)]
mod tests {
    use super::message_box;
    use ratatui::layout::Rect;

    /// A long message grows the box and wraps instead of being cut.
    #[test]
    fn long_messages_wrap_in_a_bigger_box() {
        let screen = Rect::new(0, 0, 120, 40);
        let words = "Connection failed: 192.168.1.240:22 sent its ssh-rsa host key, but \
                     known_hosts only has its ssh-ed25519 key, so the two cannot be compared. \
                     Add the server's ssh-rsa key to C:\\Users\\me\\.ssh\\known_hosts and try again.";
        let ((w, h), text) = message_box(words, (50, 8), screen);
        assert_eq!(w, 76);
        assert!(h > 8, "{h}");
        assert_eq!(usize::from(h), text.lines.len() + 2);
        let shown: String = text.lines.iter().map(|l| l.to_string() + " ").collect();
        for word in words.split(' ') {
            assert!(shown.contains(word), "{word} missing from {shown}");
        }
        for line in &text.lines {
            assert!(line.width() <= usize::from(w) - 2, "{line}");
        }
        // Short ones keep the usual size; a narrow screen bounds the width.
        assert_eq!(message_box("ok", (50, 8), screen).0, (50, 8));
        assert_eq!(message_box(words, (50, 8), Rect::new(0, 0, 40, 40)).0.0, 40);
    }
}
