//! One-field prompts: panel filter, quick filter, copy/move filter, the tab
//! title and the in-panel search.

use crate::app::input::panel_find::NameMatch;
use crate::app::state::popup::{
    CopyMoveFilterPrompt, FilePanelFilterPrompt, PanelSearch, QuickFilterPrompt, RenameTabPrompt,
};
use crate::app::text_input::TextField;
use crate::config::localization::t;
use crate::ui::popup::kit::{self, TextBox};
use crate::ui::popup::view::{DialogView, ViewCtx};
use ratatui::{Frame, style::Color};

/// A one-field prompt: `keys` are the title and text translation keys.
fn one_field(f: &mut Frame, cx: &ViewCtx<'_>, input: &TextField, (title, text): (&str, &str)) {
    TextBox {
        size: (50, 9),
        title: t(title),
        border: kit::fg(Color::Cyan),
        body: kit::prompt_text(&t(text), input, cx.theme),
        body_style: cx.theme.popup_fg(),
    }
    .with_field(kit::field_at(&t(text)))
    .render(f, cx.area, cx.theme);
}

impl DialogView for PanelSearch {
    fn render(&self, f: &mut Frame, cx: &ViewCtx<'_>) {
        let keys = match self.how {
            NameMatch::Substring => ("prompt_panel_search_title", "prompt_panel_search_text"),
            NameMatch::Prefix => ("prompt_quick_search_title", "prompt_quick_search_text"),
        };
        one_field(f, cx, &self.query, keys);
    }
}

impl DialogView for FilePanelFilterPrompt {
    fn render(&self, f: &mut Frame, cx: &ViewCtx<'_>) {
        let keys = ("prompt_filter_title", "prompt_filter_text");
        one_field(f, cx, &self.input, keys);
    }
}

impl DialogView for CopyMoveFilterPrompt {
    fn render(&self, f: &mut Frame, cx: &ViewCtx<'_>) {
        let keys = ("prompt_filter_title", "prompt_filter_text");
        one_field(f, cx, &self.input, keys);
    }
}

impl DialogView for QuickFilterPrompt {
    fn render(&self, f: &mut Frame, cx: &ViewCtx<'_>) {
        let keys = ("prompt_quick_filter_title", "prompt_quick_filter_text");
        one_field(f, cx, &self.input, keys);
    }
}

impl DialogView for RenameTabPrompt {
    fn render(&self, f: &mut Frame, cx: &ViewCtx<'_>) {
        let keys = ("prompt_rename_tab_title", "prompt_rename_tab_text");
        one_field(f, cx, &self.input, keys);
    }
}
