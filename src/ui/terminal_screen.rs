//! A terminal screen: the visible part of the output (SGR colors applied),
//! the command and its state in the title, and a close box.

use crate::app::state::ScreenId;
use crate::app::state::terminal::{TermStatus, TerminalState};
use crate::config::localization::t;
use crate::ui::hit::{self, HitTarget};
use crate::ui::resolved_theme::ResolvedTheme;
use ansi_to_tui::IntoText as _;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Text};
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

pub fn render(f: &mut Frame, area: Rect, ts: &TerminalState, id: ScreenId, theme: &ResolvedTheme) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.panel_border))
        .title(title(ts))
        .title_bottom(Line::from(CLOSE_BOX).right_aligned());
    let inner = block.inner(area);
    let lines = ts.output.window(usize::from(inner.height), ts.scroll);
    let text = terminal_output_text(&lines, usize::from(inner.height));
    f.render_widget(Paragraph::new(text).block(block), area);
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
