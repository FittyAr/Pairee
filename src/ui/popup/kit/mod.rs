//! Shared building blocks for dialog renderers: the framed popup, message
//! boxes, focus styles, checkbox rows, button bars, separators and the
//! [`crate::app::text_input::TextField`] widget. Every prompt draws these the
//! same way, so they live here once.

mod field;
mod filter_list;
mod list_popup;

pub use field::{field_spans, template_with_field};
pub use filter_list::FilterListView;
pub use list_popup::{ListPopup, Scroll, marked};

use crate::app::text_input::TextField;
use crate::config::theme::Theme;
use crate::ui::popup::centered_rect_fixed;
use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};

/// Bordered block titled `title` over the popup background.
pub fn popup_block<'a>(title: impl Into<Line<'a>>, border: Style, theme: &Theme) -> Block<'a> {
    Block::default()
        .borders(Borders::ALL)
        .border_style(border)
        .title(title)
        .style(Style::default().bg(parse_color(&theme.popup_bg)))
}

/// Bold title in `color` (Git dialogs).
pub fn accent_title(text: impl Into<String>, color: Color) -> Span<'static> {
    Span::styled(
        text.into(),
        Style::default().fg(color).add_modifier(Modifier::BOLD),
    )
}

/// Centered dim hint line (key help at the bottom of a dialog).
pub fn hint<'a>(text: impl Into<Text<'a>>) -> Paragraph<'a> {
    Paragraph::new(text)
        .alignment(Alignment::Center)
        .style(fg(Color::DarkGray))
}

/// Clears `area`, draws a bordered block titled `title` and returns the
/// inner area.
pub fn frame_in(
    f: &mut Frame,
    area: Rect,
    title: impl Into<Line<'static>>,
    border: Style,
    theme: &Theme,
) -> Rect {
    f.render_widget(Clear, area);
    let block = popup_block(title, border, theme);
    let inner = block.inner(area);
    f.render_widget(block, area);
    inner
}

/// [`frame_in`] over a centered `width` × `height` area.
pub fn dialog_frame(
    f: &mut Frame,
    size: Rect,
    (width, height): (u16, u16),
    title: impl Into<Line<'static>>,
    border: Style,
    theme: &Theme,
) -> Rect {
    frame_in(
        f,
        centered_rect_fixed(width, height, size),
        title,
        border,
        theme,
    )
}

/// A centered popup made of one paragraph: what it says and how it looks.
pub struct TextBox<'a> {
    pub size: (u16, u16),
    pub title: String,
    pub border: Style,
    pub body: Text<'a>,
    pub body_style: Style,
}

impl TextBox<'_> {
    /// Clears the area and draws `body` (in `body_style`) inside the block.
    pub fn render(self, f: &mut Frame, screen: Rect, theme: &Theme) {
        self.draw(f, screen, theme, false);
    }

    /// Like [`Self::render`], word-wrapping the body.
    pub fn render_wrapped(self, f: &mut Frame, screen: Rect, theme: &Theme) {
        self.draw(f, screen, theme, true);
    }

    fn draw(self, f: &mut Frame, screen: Rect, theme: &Theme, wrap: bool) {
        let area = centered_rect_fixed(self.size.0, self.size.1, screen);
        f.render_widget(Clear, area);
        let mut paragraph = Paragraph::new(self.body)
            .block(popup_block(self.title, self.border, theme))
            .style(self.body_style);
        if wrap {
            paragraph = paragraph.wrap(Wrap { trim: true });
        }
        f.render_widget(paragraph, area);
    }
}

/// A one-field prompt body: `template` with its first `{}` replaced by the
/// (always focused) `field`, in the popup foreground with a reversed cursor.
pub fn prompt_text(template: &str, field: &TextField, theme: &Theme) -> Text<'static> {
    let style = popup_fg(theme);
    template_with_field(
        template,
        field,
        style,
        style.add_modifier(Modifier::REVERSED),
        true,
    )
}

/// "1 item" / "N items" label: `single_key` gets the file name of the only
/// path, `plural_key` the count.
pub fn items_label(paths: &[std::path::PathBuf], single_key: &str, plural_key: &str) -> String {
    match paths {
        [single] => crate::config::localization::t(single_key).replacen(
            "{}",
            &crate::fs::file_name_lossy(single),
            1,
        ),
        many => {
            crate::config::localization::t(plural_key).replacen("{}", &many.len().to_string(), 1)
        }
    }
}

/// Theme selection colors (highlighted list rows, focused buttons).
pub fn selection(theme: &Theme) -> Style {
    Style::default()
        .bg(parse_color(&theme.selection_bg))
        .fg(parse_color(&theme.selection_fg))
}

/// A [`TextField`] in its own bordered box (Git dialogs): yellow border and
/// bold text with a cursor when focused, dim border otherwise.
pub fn input_box(field: &TextField, focused: bool, theme: &Theme) -> Paragraph<'static> {
    let (border, style) = if focused {
        (
            Color::Yellow,
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        (Color::DarkGray, popup_fg(theme))
    };
    let spans = field_spans(field, style, selection(theme), focused);
    Paragraph::new(Line::from(spans)).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(fg(border)),
    )
}

/// Style with the given foreground (borders, hints).
pub fn fg(color: Color) -> Style {
    Style::default().fg(color)
}

/// Theme popup foreground.
pub fn popup_fg(theme: &Theme) -> Style {
    Style::default().fg(parse_color(&theme.popup_fg))
}

/// Focused / unfocused styles for dialog rows and buttons.
#[derive(Debug, Clone, Copy)]
pub struct FocusStyles {
    pub active: Style,
    pub normal: Style,
}

impl FocusStyles {
    /// Cyan-on-black focus over the theme's popup foreground.
    pub fn from_theme(theme: &Theme) -> Self {
        Self {
            active: Style::default().bg(Color::Cyan).fg(Color::Black),
            normal: popup_fg(theme),
        }
    }

    /// Style of row `row` when `focus` is the focused row.
    pub fn row(&self, focus: usize, row: usize) -> Style {
        self.pick(focus == row)
    }

    pub fn pick(&self, focused: bool) -> Style {
        if focused { self.active } else { self.normal }
    }

    /// Block cursor drawn inside a focused (active-styled) text field.
    pub fn cursor(&self) -> Style {
        self.active.add_modifier(Modifier::REVERSED)
    }
}

/// `"► text"` when focused, `"  text"` otherwise, in the focus style.
pub fn marked_row<'a>(text: &str, focused: bool, styles: FocusStyles) -> Paragraph<'a> {
    let marker = if focused { "► " } else { "  " };
    Paragraph::new(format!("{marker}{text}")).style(styles.pick(focused))
}

/// Two lines: `"► label"` and `"   > field"`, the field with a cursor when
/// focused.
pub fn labelled_field<'a>(
    label: &str,
    field: &TextField,
    focused: bool,
    styles: FocusStyles,
) -> Paragraph<'a> {
    let style = styles.pick(focused);
    let marker = if focused { "► " } else { "  " };
    let mut value = vec![Span::styled("   > ", style)];
    value.extend(field_spans(field, style, styles.cursor(), focused));
    Paragraph::new(vec![
        Line::from(format!("{marker}{label}")),
        Line::from(value),
    ])
    .style(style)
}

/// `[x]` / `[ ]`.
pub fn checkbox(checked: bool) -> &'static str {
    if checked { "[x]" } else { "[ ]" }
}

/// `"[x] label"` row.
pub fn checkbox_row(checked: bool, label: &str) -> String {
    format!("{} {}", checkbox(checked), label)
}

/// Centered `[ A ]  [ B ]  ...` bar; `focused` is the index of the focused
/// button inside `labels` (if any).
pub fn button_bar<'a>(
    labels: &[String],
    focused: Option<usize>,
    styles: FocusStyles,
) -> Paragraph<'a> {
    let mut spans = Vec::with_capacity(labels.len() * 2);
    for (i, label) in labels.iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw("  "));
        }
        spans.push(Span::styled(label.clone(), styles.pick(focused == Some(i))));
    }
    Paragraph::new(Line::from(spans)).alignment(Alignment::Center)
}

/// A full-width horizontal rule.
pub fn separator<'a>(width: u16, style: Style) -> Paragraph<'a> {
    Paragraph::new(ratatui::symbols::line::HORIZONTAL.repeat(width as usize)).style(style)
}
