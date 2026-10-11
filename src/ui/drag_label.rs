//! The label that follows the pointer while files are dragged: what moves
//! where, or that the spot under the pointer takes no drop.

use crate::app::mouse::file_drag::{Drop, drop_target};
use crate::app::state::AppState;
use crate::app::state::popup::TransferPromptOp;
use crate::config::localization::t;
use crate::config::theme::Theme;
use crate::ui::text_width::display_width;
use crate::ui::theme_apply::parse_color;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Clear, Paragraph};

pub fn render(f: &mut Frame, state: &AppState, theme: &Theme) {
    let Some(drag) = state.mouse.file_drag() else {
        return;
    };
    let Some((x, y)) = drag.pointer() else {
        return;
    };
    let target = state.hits.at(x, y).map(|hit| hit.target);
    let what = match drag.paths() {
        [one] => one
            .file_name()
            .map_or_else(String::new, |name| name.to_string_lossy().into_owned()),
        many => t("drag_items").replace("{}", &many.len().to_string()),
    };
    let op = t(match drag.op() {
        TransferPromptOp::Copy => "drag_copy",
        TransferPromptOp::Move => "drag_move",
    });
    let selected = Style::default()
        .bg(parse_color(&theme.selection_bg))
        .fg(parse_color(&theme.selection_fg));
    let (text, style) = match drop_target(state, drag, target.as_ref()) {
        Some(Drop::Folder { dir, .. }) => {
            let name = dir
                .file_name()
                .map_or_else(|| dir.to_string_lossy(), |name| name.to_string_lossy());
            (
                format!(" {op} {what} → {name} "),
                selected.add_modifier(Modifier::BOLD),
            )
        }
        Some(Drop::CommandLine) => (format!(" {what} → {} ", t("drag_command_line")), selected),
        None => (
            format!(" {what} "),
            Style::default().bg(Color::DarkGray).fg(Color::White),
        ),
    };
    let screen = f.area();
    let width = (display_width(&text) as u16).min(screen.width);
    let x = (x + 2).min(screen.right().saturating_sub(width));
    let area = Rect::new(x, y, width, 1).intersection(screen);
    f.render_widget(Clear, area);
    f.render_widget(Paragraph::new(text).style(style), area);
}
