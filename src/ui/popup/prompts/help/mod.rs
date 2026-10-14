mod markdown;

pub use crate::ui::wrap::wrap_lines;
pub use markdown::parse_markdown_to_lines;

use super::super::centered_rect_in;
use crate::app::state::popup::Help;
use crate::config::localization::t;
use crate::ui::popup::view::{DialogView, ViewCtx};
use crate::ui::scrollbar::{self, ScrollTargetId, ScrollbarSurface, ScrollbarUiState};
use crate::ui::scrollbar::ScrollTarget;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
};

use crate::ui::resolved_theme::ResolvedTheme;

impl DialogView for Help {
    fn render(&self, f: &mut Frame, cx: &ViewCtx<'_>) {
        render(f, self, cx.theme, cx.area, cx.scrollbar());
    }
}

fn render(
    f: &mut Frame,
    help: &Help,
    theme: &ResolvedTheme,
    size: Rect,
    scrollbar: Option<&ScrollbarUiState>,
) {
    let Help {
        mode,
        docs,
        plugin_docs,
        active_tab,
        cursor_idx,
        scroll_y,
        active_content,
    } = help;
    let area = centered_rect_in(90, 85, size); // Expand to 90% width, 85% height
    f.render_widget(Clear, area);

    // The key hint gets the bottom row, under both panes (not over them).
    let [panes_area, hint_area] =
        Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).areas(area);
    // Split into Left (list) and Right (content viewer)
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25), // 25% for document list
            Constraint::Percentage(75), // 75% for content
        ])
        .split(panes_area);

    // 1. Render Left panel (document selection list)
    let current_docs = if *active_tab == 0 { docs } else { plugin_docs };
    let list = DocList {
        focused: *mode == 0,
        active_tab: *active_tab,
        docs: current_docs,
        cursor_idx: *cursor_idx,
    };
    render_doc_list(f, chunks[0], theme, &list);

    // 2. Render Right panel (content viewer)
    let doc_title = current_docs
        .get(*cursor_idx)
        .map_or_else(|| t("help_documentation"), |(title, _)| title.clone());
    let right_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(focus_color(*mode == 1, theme)))
        .title(format!(" {} ", doc_title))
        .style(Style::default().bg(theme.popup_bg));
    match active_content {
        Some(content) => render_content(
            f,
            chunks[1],
            theme,
            right_block,
            content,
            *scroll_y,
            scrollbar,
        ),
        None => {
            let empty_paragraph = Paragraph::new(t("help_no_document")).block(right_block);
            f.render_widget(empty_paragraph, chunks[1]);
        }
    }

    // 3. Render help hint at the bottom
    let hint_text = t("help_hint");
    f.render_widget(
        Paragraph::new(hint_text)
            .alignment(ratatui::layout::Alignment::Center)
            .style(Style::default().fg(Color::DarkGray)),
        hint_area,
    );
}

/// Yellow border on the focused pane, the theme border otherwise.
fn focus_color(focused: bool, theme: &ResolvedTheme) -> Color {
    if focused {
        Color::Yellow
    } else {
        theme.popup_border
    }
}

/// What the left pane shows: the tab and its documents.
struct DocList<'a> {
    focused: bool,
    active_tab: usize,
    docs: &'a [(String, std::path::PathBuf)],
    cursor_idx: usize,
}

/// Core/Plugins tab bar above the document titles.
fn render_doc_list(f: &mut Frame, left_area: Rect, theme: &ResolvedTheme, list: &DocList) {
    let border_style = Style::default().fg(focus_color(list.focused, theme));
    let bg_style = Style::default().bg(theme.popup_bg);
    // Split left panel into Tabs and Document list
    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Border + Tab bar
            Constraint::Min(1),    // List of documents
        ])
        .split(left_area);

    let dim = Style::default().fg(Color::DarkGray);
    let tab_style = |idx: usize| {
        if list.active_tab == idx {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(ratatui::style::Modifier::BOLD)
        } else {
            dim
        }
    };
    let tabs_line = Line::from(vec![
        Span::styled(" [ ", dim),
        Span::styled(t("help_tab_core"), tab_style(0)),
        Span::styled(" ]  [ ", dim),
        Span::styled(t("help_tab_plugins"), tab_style(1)),
        Span::styled(" ]", dim),
    ]);
    let tab_block = Block::default()
        .borders(Borders::TOP | Borders::LEFT | Borders::RIGHT)
        .border_style(border_style)
        .style(bg_style);
    f.render_widget(Paragraph::new(tabs_line).block(tab_block), left_chunks[0]);

    let rows_area = left_block_inner(left_chunks[1]);
    // Scrolled to keep the cursor in view on short screens.
    let start = crate::ui::scrollbar::centered_scroll(
        list.cursor_idx,
        list.docs.len(),
        rows_area.height as usize,
    );
    let list_items: Vec<ListItem> = list
        .docs
        .iter()
        .enumerate()
        .skip(start)
        .map(|(i, (doc_title, _))| {
            let style = if i == list.cursor_idx {
                theme
                    .selection()
                    .add_modifier(ratatui::style::Modifier::BOLD)
            } else {
                Style::default().fg(theme.popup_fg)
            };
            ListItem::new(Line::from(vec![Span::styled(
                format!("  {}  ", doc_title),
                style,
            )]))
        })
        .collect();
    let left_block = Block::default()
        .borders(Borders::BOTTOM | Borders::LEFT | Borders::RIGHT)
        .border_style(border_style)
        .style(bg_style);
    let doc_list = List::new(list_items).block(left_block).style(bg_style);
    f.render_widget(doc_list, left_chunks[1]);
    register_doc_list(left_chunks[0], (rows_area, start), list);
}

/// Inside of the list block (borders left, right and bottom).
fn left_block_inner(area: Rect) -> Rect {
    Rect {
        x: area.x + 1,
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(1),
        ..area
    }
}

/// Mouse targets of the left pane: the two tabs (Left / Right switch
/// them) and the documents (Tab first when the reader has the focus).
fn register_doc_list(tabs_area: Rect, (rows_area, start): (Rect, usize), list: &DocList) {
    use crate::ui::popup::kit::mouse;
    use crossterm::event::{KeyCode, KeyEvent};
    let focus: Vec<KeyEvent> = if list.focused {
        Vec::new()
    } else {
        mouse::key(KeyCode::Tab)
    };
    let tabs_row = Rect {
        x: tabs_area.x + 1,
        y: tabs_area.y + 1,
        width: tabs_area.width.saturating_sub(2),
        height: 1,
    };
    let labels = [
        format!(" [ {} ]", t("help_tab_core")),
        format!("  [ {} ]", t("help_tab_plugins")),
    ];
    let keys = (0..2)
        .map(|tab| {
            let mut keys = focus.clone();
            if tab != list.active_tab {
                keys.push(KeyEvent::from(KeyCode::Left));
            }
            keys
        })
        .collect();
    mouse::span_buttons(tabs_row, &labels, (0, false), keys);
    mouse::focused_list_rows(rows_area, (start, list.docs.len(), list.cursor_idx), &focus);
}

/// The document as wrapped Markdown, with a scrollbar when it overflows.
fn render_content(
    f: &mut Frame,
    right_area: Rect,
    theme: &ResolvedTheme,
    right_block: Block,
    content: &str,
    scroll_y: usize,
    scrollbar: Option<&ScrollbarUiState>,
) {
    let parsed_lines = parse_markdown_to_lines(content);
    let inner_width = (right_area.width.saturating_sub(4)) as usize;
    let wrapped_lines = wrap_lines(parsed_lines, inner_width);
    let inner = right_block.inner(right_area);
    f.render_widget(right_block, right_area);
    scrollbar::render_text_pane(
        f,
        inner,
        (wrapped_lines, Style::default().fg(theme.popup_fg)),
        scroll_y,
        theme,
        ScrollTarget {
            surface: ScrollbarSurface::Popup,
            hits: scrollbar,
            id: ScrollTargetId::HelpContent,
        },
    );
}
