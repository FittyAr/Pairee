//! Shared Copy / Move dialog renderer; the operation only changes the labels
//! ([`crate::app::state::popup::TransferPromptOp::labels`]).

mod input;
mod options;

use crate::app::state::CopyMovePromptState as Prompt;
use crate::config::localization::t;
use crate::ui::popup::kit::{self, FocusStyles};
use crate::ui::popup::view::{DialogView, ViewCtx};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
};

impl DialogView for Prompt {
    fn render(&self, f: &mut Frame, cx: &ViewCtx<'_>) {
        render(f, self, cx.theme, cx.area);
    }
}

fn render(
    f: &mut Frame,
    prompt: &Prompt,
    theme: &crate::ui::resolved_theme::ResolvedTheme,
    size: Rect,
) {
    let labels = prompt.op.labels();
    let inner = kit::dialog_frame(
        f,
        size,
        (75, 17),
        t(labels.title),
        Style::default().fg(theme.ui.warning),
        theme,
    );

    let mut constraints = vec![Constraint::Length(2)]; // input
    constraints.extend([Constraint::Length(1); 13]); // rows, separators, buttons
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(inner);

    let styles = FocusStyles::from_theme(theme);
    input::render_input(f, chunks[0], prompt, styles, theme);
    options::render_options(f, &chunks, prompt, styles, (inner.width, theme));

    let buttons = vec![
        t(labels.button),
        t("btn_f10_tree"),
        t("btn_filter"),
        t("btn_cancel_bracket"),
    ];
    kit::form_buttons(
        f,
        chunks[13],
        &buttons,
        kit::mouse::FormFocus::new(Prompt::FORM, prompt.cursor_idx),
        styles,
    );
}
