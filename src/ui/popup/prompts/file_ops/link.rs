use crate::app::state::LinkKind;
use crate::app::state::popup::CreateLinkPrompt;
use crate::config::localization::t;
use crate::ui::popup::kit;
use crate::ui::popup::view::{DialogView, ViewCtx};
use ratatui::Frame;

impl DialogView for CreateLinkPrompt {
    fn render(&self, f: &mut Frame, cx: &ViewCtx<'_>) {
        let (theme, size) = (cx.theme, cx.area);
        let CreateLinkPrompt {
            src,
            dest_input,
            kind,
        } = self;
        let title = match kind {
            LinkKind::Symbolic => t("prompt_symlink_title"),
            LinkKind::Hard => t("prompt_hardlink_title"),
        };
        let template = t("prompt_link_text").replacen("{}", &crate::fs::file_name_lossy(src), 1);
        kit::PromptBox {
            size: (60, 9),
            title,
            border: kit::fg(theme.ui.warning),
            template: &template,
            field: dest_input,
        }
        .render(f, size, theme);
    }
}
