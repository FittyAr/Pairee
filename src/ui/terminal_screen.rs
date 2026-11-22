//! A terminal screen: the emulated terminal a program draws on (or, for
//! Transfer Engine jobs, the output lines with their SGR colors), the
//! command and its state in the title, what the keys do at the bottom, and
//! a close box.

use crate::app::state::ScreenId;
use crate::app::state::terminal::{TermStatus, TerminalState};
use crate::config::localization::t;
use crate::ui::hit::{self, HitTarget};
use crate::ui::resolved_theme::ResolvedTheme;
use ansi_to_tui::IntoText as _;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph};

/// The close box drawn at the right of the bottom border.
const CLOSE_BOX: &str = "[×]";

/// `lines` as ratatui Text with SGR colors applied, at most the last
/// `max_lines` of them.
pub fn terminal_output_text(lines: &[String], max_lines: usize) -> Text<'static> {
    if lines.is_empty() || max_lines == 0 {
        return Text::default();
    }
    let start = lines.len().saturating_sub(max_lines);
    let joined = lines[start..].join("\n");
    joined.into_text().unwrap_or_else(|_| Text::from(joined))
}

/// The emulated screen as text: each cell with its colors and attributes.
fn screen_text(screen: &vt100::Screen) -> Text<'static> {
    let (rows, cols) = screen.size();
    let lines = (0..rows.get())
        .map(|row| {
            let mut spans: Vec<Span<'static>> = Vec::new();
            for col in 0..cols.get() {
                let Some(cell) = screen.cell(row, col) else {
                    continue;
                };
                if cell.is_wide_continuation() {
                    continue;
                }
                let text = match cell.contents() {
                    "" => " ",
                    text => text,
                };
                let style = cell_style(cell);
                match spans.last_mut() {
                    Some(last) if last.style == style => last.content.to_mut().push_str(text),
                    _ => spans.push(Span::styled(text.to_string(), style)),
                }
            }
            Line::from(spans)
        })
        .collect::<Vec<_>>();
    Text::from(lines)
}

fn cell_style(cell: &vt100::Cell) -> Style {
    let mut style = Style::default()
        .fg(color(cell.fgcolor()))
        .bg(color(cell.bgcolor()));
    for (on, modifier) in [
        (cell.bold(), Modifier::BOLD),
        (cell.dim(), Modifier::DIM),
        (cell.italic(), Modifier::ITALIC),
        (cell.underline(), Modifier::UNDERLINED),
        (cell.inverse(), Modifier::REVERSED),
    ] {
        if on {
            style = style.add_modifier(modifier);
        }
    }
    style
}

fn color(color: vt100::Color) -> Color {
    match color {
        vt100::Color::Default => Color::Reset,
        vt100::Color::Idx(i) => Color::Indexed(i),
        vt100::Color::Rgb(r, g, b) => Color::Rgb(r, g, b),
    }
}

/// Title: command, process state (pid while running) and scroll position.
fn title(ts: &TerminalState) -> String {
    let status = match ts.status {
        TermStatus::Running => match ts.pid() {
            Some(pid) => format!(
                "{} · {}",
                t("terminal_status_running"),
                t("terminal_status_pid").replacen("{}", &pid.to_string(), 1)
            ),
            None => t("terminal_status_running"),
        },
        TermStatus::Exited(true) => t("terminal_status_done"),
        TermStatus::Exited(false) => t("terminal_status_failed"),
    };
    let mut title = format!(
        "{}({status}) ",
        t("terminal_title").replacen("{}", &ts.command, 1)
    );
    if ts.scroll > 0 {
        let scrolled = t("terminal_status_scrolled").replacen("{}", &ts.scroll.to_string(), 1);
        title.push_str(&format!("[{scrolled}] "));
    }
    title
}

/// How a terminal screen is painted besides its state.
pub struct TerminalView<'a> {
    pub id: ScreenId,
    pub theme: &'a ResolvedTheme,
    /// What the keys do, on the bottom border.
    pub hint: &'a str,
    /// Show the program's cursor (the screen has the keyboard).
    pub focused: bool,
}

pub fn render(f: &mut Frame, area: Rect, ts: &TerminalState, view: &TerminalView) {
    let TerminalView {
        id, theme, hint, ..
    } = *view;
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.panel_border))
        .title(title(ts))
        .title_bottom(Line::from(format!(" {hint} ")))
        .title_bottom(Line::from(CLOSE_BOX).right_aligned());
    let inner = block.inner(area);
    match &ts.vt {
        Some(vt) => {
            ts.fit(inner.height, inner.width);
            let (text, cursor) = vt.with_screen(ts.scroll, |screen| {
                let cursor = (!screen.hide_cursor()).then(|| screen.cursor_position());
                (screen_text(screen), cursor)
            });
            f.render_widget(Paragraph::new(text).block(block), area);
            if let Some((row, col)) =
                cursor.filter(|_| view.focused && ts.is_running() && ts.scroll == 0)
            {
                f.set_cursor_position((inner.x + col, inner.y + row));
            }
        }
        None => {
            let lines = ts.output.window(usize::from(inner.height), ts.scroll);
            let text = terminal_output_text(&lines, usize::from(inner.height));
            f.render_widget(Paragraph::new(text).block(block), area);
        }
    }
    hit::register(area, HitTarget::TerminalOutput(id));
    let close_width = CLOSE_BOX.chars().count() as u16;
    if area.width > close_width + 2 {
        // On the bottom border: the top row reveals a hidden menu bar.
        let close = Rect::new(
            area.right() - close_width - 1,
            area.bottom().saturating_sub(1),
            close_width,
            1,
        );
        hit::register(close, HitTarget::CloseScreen(id));
    }
}
