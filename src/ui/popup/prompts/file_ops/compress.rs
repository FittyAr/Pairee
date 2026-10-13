use crate::app::state::popup::CompressPrompt;
use crate::config::localization::t;
use crate::ui::popup::kit::{self, TextBox};
use crate::ui::popup::view::{DialogView, ViewCtx};
use ratatui::{Frame, style::Color};

impl DialogView for CompressPrompt {
    fn render(&self, f: &mut Frame, cx: &ViewCtx<'_>) {
        let (theme, size) = (cx.theme, cx.area);
        let CompressPrompt {
            input,
            targets,
            dest_dir,
        } = self;
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
            body_style: theme.popup_fg(),
        }
        .with_field(kit::field_at(&template))
        .render(f, size, theme);
    }
}
