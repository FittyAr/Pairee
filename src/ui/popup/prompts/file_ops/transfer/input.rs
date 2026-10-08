use crate::app::state::CopyMovePromptState as Prompt;
use crate::config::localization::t;
use crate::ui::popup::kit::{self, FocusStyles};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::Paragraph,
};

/// "Copy N items to:" plus the destination field (with the history
/// suggestion greyed after the cursor when the field is focused).
pub fn render_input(f: &mut Frame, area: Rect, prompt: &Prompt, styles: FocusStyles) {
    let labels = prompt.op.labels();
    let label = kit::items_label(&prompt.src_paths, labels.single, labels.plural);
    let focused = prompt.cursor_idx == Prompt::ROW_INPUT;
    let style = styles.pick(focused);
    let mut spans = kit::field_spans(&prompt.input, style, styles.cursor(), focused);
    if focused {
        let input = prompt.input.text();
        if let Some(suffix) = crate::fs::transfer::history::suggest_destination(input)
            .and_then(|s| s.get(input.len()..).map(str::to_string))
            .filter(|s| !s.is_empty())
        {
            spans.push(Span::styled(suffix, Style::default().fg(Color::DarkGray)));
        }
    }
    let lines = vec![
        Line::from(format!("{} {}", label, t(labels.to))),
        Line::from(spans),
    ];
    f.render_widget(Paragraph::new(Text::from(lines)), area);
}
