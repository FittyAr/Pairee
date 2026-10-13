//! The command line: a prompt (screen and clipboard indicators, the active
//! folder) and the typed text with its cursor, scrolled sideways so what is
//! being typed stays visible however long the prompt or the text are.

use crate::app::context::AppContext;
use crate::app::state::AppState;
use crate::app::state::file_clipboard::ClipMode;
use crate::config::localization::t;
use crate::ui::popup::kit::field_spans_scrolled;
use crate::ui::text_width::{display_width, skip_columns};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

/// The prompt never takes more than this share (1/N) of the line while the
/// line is in use, so the text always has room.
const PROMPT_SHARE: usize = 2;

const ELLIPSIS: &str = "…";

pub fn render_cli(f: &mut Frame, area: Rect, state: &AppState, context: &AppContext) {
    let theme = &context.theme();
    let width = usize::from(area.width);
    let prompt = prompt(state);
    let editing = state.cli.is_active();
    let prompt = if editing {
        shorten_left(&prompt, (width / PROMPT_SHARE).max(1))
    } else {
        prompt
    };
    let text_style = Style::default().fg(theme.cli_fg);
    let room = width.saturating_sub(display_width(&prompt));
    let mut spans = vec![Span::styled(prompt, Style::default().fg(theme.header_fg))];
    spans.extend(field_spans_scrolled(
        &state.cli.field,
        text_style,
        text_style.add_modifier(Modifier::REVERSED),
        editing,
        room,
    ));
    let paragraph = Paragraph::new(Line::from(spans)).style(Style::default().bg(theme.cli_bg));
    f.render_widget(paragraph, area);
    crate::ui::hit::register(
        area,
        crate::ui::hit::HitTarget::Action(crate::keybindings::Action::FocusCli),
    );
}

/// Screens and clipboard indicators, the active folder and the shell sign.
fn prompt(state: &AppState) -> String {
    let prompt_symbol = if cfg!(target_os = "windows") {
        ">"
    } else {
        "$ "
    };
    let screen_indicator = if state.screens.len() > 1 {
        t("cli_screens_indicator")
            .replacen("{}", &(state.screens.active_index() + 1).to_string(), 1)
            .replacen("{}", &state.screens.len().to_string(), 1)
    } else {
        String::new()
    };
    let clipboard_indicator = state
        .file_clipboard
        .as_ref()
        .map(|clip| {
            let key = match clip.mode {
                ClipMode::Copy => "clipboard_indicator_copy",
                ClipMode::Cut => "clipboard_indicator_cut",
            };
            format!("{} ", t(key).replace("{}", &clip.paths.len().to_string()))
        })
        .unwrap_or_default();
    format!(
        "{screen_indicator}{clipboard_indicator}{}{prompt_symbol}",
        state.get_active_panel().current_path.to_string_lossy()
    )
}

/// `text` cut from the left to `max` columns, with `…` in front when cut
/// (the end of a long path is the part that tells folders apart).
fn shorten_left(text: &str, max: usize) -> String {
    let width = display_width(text);
    if width <= max {
        return text.to_string();
    }
    let keep = max.saturating_sub(display_width(ELLIPSIS));
    format!("{ELLIPSIS}{}", skip_columns(text, width - keep))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn long_prompts_keep_their_end() {
        assert_eq!(shorten_left("/very/long/path>", 8), "…g/path>");
        assert_eq!(shorten_left("short>", 8), "short>");
    }
}
