use crate::app::state::PopupType;
use crate::config::localization::t;
use crate::ui::popup::kit::{self, TextBox};
use ratatui::{Frame, layout::Rect, style::Color};

pub fn render(
    f: &mut Frame,
    popup: &PopupType,
    theme: &crate::config::theme::Theme,
    size: Rect,
) -> bool {
    let PopupType::CompressPrompt {
        input,
        targets,
        dest_dir,
    } = popup
    else {
        return false;
    };
    let src_label = kit::items_label(targets, "prompt_compress_sing", "prompt_compress_plur");
    let template = format!(
        "\n {}\n {}\n\n > {{}}.zip\n\n {}",
        src_label,
        t("prompt_copy_dest").replacen("{}", &dest_dir.to_string_lossy(), 1),
        t("prompt_confirm_cancel_hint")
    );
    TextBox {
        size: (60, 9),
        title: t("prompt_compress_title"),
        border: kit::fg(Color::Yellow),
        body: kit::prompt_text(&template, input, theme),
        body_style: kit::popup_fg(theme),
    }
    .render(f, size, theme);
    true
}
