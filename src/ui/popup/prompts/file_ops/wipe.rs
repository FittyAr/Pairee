use crate::app::state::popup::WipeConfirm;
use crate::config::localization::t;
use crate::ui::popup::kit;
use crate::ui::popup::view::{DialogView, ViewCtx};
use ratatui::{Frame, style::Color};

impl DialogView for WipeConfirm {
    fn render(&self, f: &mut Frame, cx: &ViewCtx<'_>) {
        let (theme, size) = (cx.theme, cx.area);
        let WipeConfirm { paths } = self;
        kit::TextBox {
            size: (55, 8),
            title: t("prompt_wipe_warn_title"),
            border: kit::fg(Color::Red),
            body: t("prompt_wipe_warn_text")
                .replacen("{}", &paths.len().to_string(), 1)
                .into(),
            body_style: kit::fg(Color::LightRed),
        }
        .render(f, size, theme);
    }
}
