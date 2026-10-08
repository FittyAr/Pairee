//! Shared Copy / Move dialog renderer; the operation only changes the labels
//! ([`crate::app::state::popup::TransferPromptOp::labels`]).

mod input;
mod options;

use crate::app::state::CopyMovePromptState as Prompt;
use crate::config::localization::t;
use crate::ui::popup::kit::{self, FocusStyles};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
};

pub fn render(
    f: &mut Frame,
    prompt: &Prompt,
    theme: &crate::config::theme::Theme,
    size: Rect,
) -> bool {
    let labels = prompt.op.labels();
    let inner = kit::dialog_frame(
        f,
        size,
        (75, 17),
        t(labels.title),
        Style::default().fg(Color::Yellow),
        theme,
    );

    let mut constraints = vec![Constraint::Length(2)]; // input
    constraints.extend([Constraint::Length(1); 13]); // rows, separators, buttons
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(inner);

    let styles = FocusStyles::from_theme(theme);
    input::render_input(f, chunks[0], prompt, styles);
    options::render_options(f, &chunks, prompt, styles, inner.width);

    let buttons = vec![
        t(labels.button),
        t("btn_f10_tree"),
        t("btn_filter"),
        t("btn_cancel_bracket"),
    ];
    f.render_widget(
        kit::button_bar(
            &buttons,
            kit::focused_button(prompt.cursor_idx, Prompt::BUTTON_SUBMIT),
            styles,
        ),
        chunks[13],
    );
    true
}
