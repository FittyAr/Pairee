use super::line_spans::{LineMarks, LineStyles, line_spans};
use crate::app::editor::EditorState;
use crate::app::editor::search::Matcher;
use crate::app::editor::viewport::Viewport;
use crate::app::state::PopupType;
use crate::app::text_input;
use crate::config::localization::t;
use crate::syntax::LineTokens;
use crate::ui::resolved_theme::ResolvedTheme;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};
use unicode_segmentation::UnicodeSegmentation;

/// Digits of the line numbers: at least 4, more for longer files.
fn number_digits(line_count: usize) -> usize {
    line_count.to_string().len().max(4)
}

/// Columns of the line-number gutter (`"1234 │ "`).
fn gutter_width(line_count: usize) -> u16 {
    (number_digits(line_count) + 3) as u16
}

/// Screen row of the cursor line, or `None` while it is scrolled out of
/// the `height` rows shown from `scroll_y` (e.g. by the mouse wheel).
fn cursor_row(cursor_y: usize, scroll_y: usize, height: u16) -> Option<u16> {
    let row = cursor_y.checked_sub(scroll_y)?;
    (row < height as usize).then_some(row as u16)
}

/// Display settings of the editor screen.
#[derive(Debug, Clone, Copy)]
pub struct EditorView {
    pub show_line_numbers: bool,
    /// Paint syntax colors (the setting; the file may still have none).
    pub syntax: bool,
}

pub fn render_editor_widget(
    f: &mut Frame,
    area: Rect,
    ed: &EditorState,
    view: EditorView,
    theme: &ResolvedTheme,
    active_popup: Option<&PopupType>,
) {
    let block = editor_block(ed, theme);
    let inner = block.inner(area);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);
    let edit_area = chunks[0];

    // Horizontal scroll: keep the cursor column inside the text area.
    let gutter = if view.show_line_numbers {
        gutter_width(ed.lines.len())
    } else {
        0
    };
    let text_width = edit_area.width.saturating_sub(gutter).max(1) as usize;
    let cursor_col = ed.cursor_column();
    let scroll_x = (cursor_col + 1).saturating_sub(text_width);
    ed.viewport.set(Viewport {
        x: edit_area.x + gutter,
        y: edit_area.y,
        width: text_width as u16,
        height: edit_area.height,
        scroll_x,
    });

    let text = text_lines(ed, view, theme, active_popup, scroll_x);
    f.render_widget(block, area);
    f.render_widget(
        Paragraph::new(text).style(Style::default().fg(theme.panel_fg)),
        edit_area,
    );

    let status_para = Paragraph::new(status_text(ed))
        .style(Style::default().bg(theme.header_fg).fg(theme.header_bg));
    f.render_widget(status_para, chunks[1]);

    // Draw the terminal blinking cursor at the editing position
    let cursor_x = edit_area.x + gutter + (cursor_col - scroll_x) as u16;
    if let Some(row) = cursor_row(ed.cursor_y, ed.scroll_y, edit_area.height)
        && cursor_x < edit_area.right()
    {
        f.set_cursor_position((cursor_x, edit_area.y + row));
    }
}

/// The search whose matches are highlighted: the one being typed in the
/// find dialog, else the last one run (until Esc).
fn shown_search(ed: &EditorState, active_popup: Option<&PopupType>) -> Option<Matcher> {
    match active_popup {
        Some(PopupType::EditorSearchPrompt(dialog)) => dialog.matcher(),
        _ => ed
            .show_matches
            .then(|| ed.last_search.as_ref().map(|s| s.matcher.clone()))
            .flatten(),
    }
}

/// The visible lines: line numbers, syntax colors, matches and selection.
fn text_lines(
    ed: &EditorState,
    view: EditorView,
    theme: &ResolvedTheme,
    active_popup: Option<&PopupType>,
    scroll_x: usize,
) -> Vec<Line<'static>> {
    let height = ed.viewport.get().height as usize;
    let visible = ed.scroll_y..(ed.scroll_y + height).min(ed.lines.len()).max(ed.scroll_y);
    let mut syntax = ed.syntax.borrow_mut();
    let tokens: &[LineTokens] = match syntax.as_mut().filter(|_| view.syntax) {
        Some(cache) => cache.highlight(visible.clone(), |r| {
            ed.lines[r].iter().map(String::as_str).collect()
        }),
        None => &[],
    };
    let matcher = shown_search(ed, active_popup);
    let styles = LineStyles {
        normal: Style::default().fg(theme.panel_fg),
        search: theme.search_match(),
        selected: theme.selection(),
        theme,
    };
    let region = ed.region();
    let digits = number_digits(ed.lines.len());
    visible
        .map(|y| {
            let matches = matcher
                .as_ref()
                .map(|m| ed.line_matches(m, y))
                .unwrap_or_default();
            let marks = LineMarks {
                region,
                matches: &matches,
                tokens: tokens.get(y - ed.scroll_y).map_or(&[], Vec::as_slice),
                tab_size: ed.tab_size,
                scroll_x,
            };
            let mut spans = Vec::new();
            if view.show_line_numbers {
                spans.push(Span::raw(format!("{:>digits$} │ ", y + 1)));
            }
            spans.extend(line_spans(y, &ed.lines[y], &marks, &styles));
            Line::from(spans)
        })
        .collect()
}

/// Frame titled with the file name and a `*` when modified.
fn editor_block<'a>(ed: &EditorState, theme: &ResolvedTheme) -> Block<'a> {
    let title = t("editor_title")
        .replacen("{}", &ed.display_path().to_string_lossy(), 1)
        .replacen("{}", if ed.is_dirty() { "*" } else { "" }, 1);
    Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.panel_border))
        .title(Span::styled(
            title,
            Style::default()
                .fg(theme.header_fg)
                .add_modifier(ratatui::style::Modifier::BOLD),
        ))
        .style(Style::default().bg(theme.panel_bg))
}

/// Line length, line count, cursor position, line ending, language and
/// mode flags.
fn status_text(ed: &EditorState) -> String {
    let current_line = ed.current_line();
    let mut flags = ed.format.line_ending.label().to_string();
    if let Some(cache) = ed.syntax.borrow().as_ref() {
        flags.push_str(" | ");
        flags.push_str(cache.language());
    }
    for (on, key) in [
        (ed.stamp.read_only, "editor_read_only_flag"),
        (ed.block_mode, "editor_block_mode_flag"),
    ] {
        if on {
            flags.push_str(" | ");
            flags.push_str(&t(key));
        }
    }
    t("editor_status_text")
        .replacen("{}", &current_line.graphemes(true).count().to_string(), 1)
        .replacen("{}", &ed.lines.len().to_string(), 1)
        .replacen("{}", &(ed.cursor_y + 1).to_string(), 1)
        .replacen(
            "{}",
            &(text_input::grapheme_col(current_line, ed.cursor_x) + 1).to_string(),
            1,
        )
        .replacen("{}", &flags, 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gutter_grows_with_the_line_numbers() {
        assert_eq!(gutter_width(1), 7);
        assert_eq!(gutter_width(9_999), 7);
        assert_eq!(gutter_width(10_000), 8);
        let digits = number_digits(10_000);
        let label = format!("{:>digits$} │ ", 10_000);
        assert_eq!(
            crate::ui::text_width::display_width(&label),
            gutter_width(10_000) as usize
        );
    }

    #[test]
    fn no_cursor_while_its_line_is_scrolled_away() {
        assert_eq!(cursor_row(5, 3, 10), Some(2));
        assert_eq!(cursor_row(2, 3, 10), None);
        assert_eq!(cursor_row(13, 3, 10), None);
        assert_eq!(cursor_row(0, 0, 0), None);
    }
}
