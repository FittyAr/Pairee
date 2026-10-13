use crate::app::input::panel_find::NameMatch;
use crate::app::state::PopupType;
use crate::app::state::popup::CopyMoveFilterPrompt;
use crate::app::text_input::TextField;
use crate::config::localization::t;
use crate::ui::popup::kit::{self, TextBox};
use crate::ui::popup::view::{DialogView, ViewCtx};
use ratatui::{Frame, layout::Rect, style::Color};

/// One-field prompts: panel filter, quick filter, copy/move filter, the
/// tab title and the in-panel search.
pub fn render(
    f: &mut Frame,
    popup: &PopupType,
    theme: &crate::ui::resolved_theme::ResolvedTheme,
    size: Rect,
) -> bool {
    let (input, title, text) = match popup {
        PopupType::PanelSearch { query, how, .. } => match how {
            NameMatch::Substring => (
                query,
                "prompt_panel_search_title",
                "prompt_panel_search_text",
            ),
            NameMatch::Prefix => (
                query,
                "prompt_quick_search_title",
                "prompt_quick_search_text",
            ),
        },
        PopupType::FilePanelFilterPrompt { input } => {
            (input, "prompt_filter_title", "prompt_filter_text")
        }
        PopupType::QuickFilterPrompt { input, .. } => (
            input,
            "prompt_quick_filter_title",
            "prompt_quick_filter_text",
        ),
        PopupType::RenameTabPrompt { input, .. } => {
            (input, "prompt_rename_tab_title", "prompt_rename_tab_text")
        }
        _ => return false,
    };
    one_field(f, input, (title, text), theme, size);
    true
}

/// A one-field prompt: `keys` are the title and text translation keys.
fn one_field(
    f: &mut Frame,
    input: &TextField,
    (title, text): (&str, &str),
    theme: &crate::ui::resolved_theme::ResolvedTheme,
    size: Rect,
) {
    TextBox {
        size: (50, 9),
        title: t(title),
        border: kit::fg(Color::Cyan),
        body: kit::prompt_text(&t(text), input, theme),
        body_style: theme.popup_fg(),
    }
    .with_field(kit::field_at(&t(text)))
    .render(f, size, theme);
}

impl DialogView for CopyMoveFilterPrompt {
    fn render(&self, f: &mut Frame, cx: &ViewCtx<'_>) {
        let keys = ("prompt_filter_title", "prompt_filter_text");
        one_field(f, &self.input, keys, cx.theme, cx.area);
    }
}
