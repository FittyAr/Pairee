use crate::app::state::popup::DescribeFilePrompt;
use crate::config::localization::t;
use crate::ui::popup::kit::{self, TextBox};
use crate::ui::popup::view::{DialogView, ViewCtx};
use ratatui::Frame;

impl DialogView for DescribeFilePrompt {
    fn render(&self, f: &mut Frame, cx: &ViewCtx<'_>) {
        let (theme, size) = (cx.theme, cx.area);
        let DescribeFilePrompt {
            path,
            current_desc,
            input,
        } = self;
        let template = t("prompt_describe_text")
            .replacen("{}", &crate::fs::file_name_lossy(path), 1)
            .replacen("{}", current_desc, 1);
        TextBox {
            size: (60, 10),
            title: t("prompt_description_title"),
            border: kit::fg(theme.ui.accent),
            body: kit::prompt_text(&template, input, theme),
            body_style: theme.popup_fg(),
        }
        .with_field(kit::field_at(&template))
        .render(f, size, theme);
    }
}
