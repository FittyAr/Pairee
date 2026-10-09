use crate::app::context::AppContext;
use crate::app::state::AppState;
use crate::app::state::file_clipboard::ClipMode;
use crate::config::localization::t;
use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
};

pub fn render_cli(f: &mut Frame, area: Rect, state: &AppState, context: &AppContext) {
    let theme = &context.config.theme;

    // Build the dynamic prompt based on active panel path
    let active_path = &state.get_active_panel().current_path;
    let prompt_symbol = if cfg!(target_os = "windows") {
        ">"
    } else {
        "$ "
    };
    let screen_indicator = if state.screens.len() > 1 {
        format!(
            " [Screens: {}/{}] ",
            state.active_screen_idx + 1,
            state.screens.len()
        )
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

    let prompt = format!(
        "{}{}{}{}",
        screen_indicator,
        clipboard_indicator,
        active_path.to_string_lossy(),
        prompt_symbol
    );

    let cli_text = &state.cli_input;

    let line = Line::from(vec![
        Span::styled(prompt, Style::default().fg(parse_color(&theme.header_fg))),
        Span::styled(
            cli_text.clone(),
            Style::default().fg(parse_color(&theme.cli_fg)),
        ),
    ]);

    let paragraph = Paragraph::new(line).style(Style::default().bg(parse_color(&theme.cli_bg)));

    f.render_widget(paragraph, area);
}
