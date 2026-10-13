//! Shared building blocks for dialog renderers: the framed popup, message
//! boxes, focus styles, checkbox rows, button bars, separators and the
//! [`crate::app::text_input::TextField`] widget. Every prompt draws these the
//! same way, so they live here once.

mod field;
mod filter_list;
mod list_popup;
pub mod mouse;

pub use field::{field_spans, field_spans_scrolled, template_with_field};
pub use filter_list::FilterListView;
pub use list_popup::{ListPopup, Scroll, list_area, marked};

use crate::app::text_input::TextField;
use crate::ui::popup::centered_rect_fixed;
use crate::ui::resolved_theme::ResolvedTheme;
use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};

/// Bordered block titled `title` over the popup background.
pub fn popup_block<'a>(
    title: impl Into<Line<'a>>,
    border: Style,
    theme: &ResolvedTheme,
) -> Block<'a> {
    Block::default()
        .borders(Borders::ALL)
        .border_style(border)
        .title(title)
        .style(Style::default().bg(theme.popup_bg))
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
    theme: &ResolvedTheme,
) -> Rect {
    super::popup_body(area);
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
    theme: &ResolvedTheme,
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

impl<'a> TextBox<'a> {
    /// Clears the area and draws `body` (in `body_style`) inside the block.
    pub fn render(self, f: &mut Frame, screen: Rect, theme: &ResolvedTheme) {
        self.draw(f, screen, theme, false);
    }

    /// Like [`Self::render`], word-wrapping the body.
    pub fn render_wrapped(self, f: &mut Frame, screen: Rect, theme: &ResolvedTheme) {
        self.draw(f, screen, theme, true);
    }

    /// The body holds the prompt's text field at `at` (see [`field_at`]):
    /// a click on that row puts the field's cursor under the pointer.
    pub fn with_field(self, at: (u16, u16)) -> FieldBox<'a> {
        FieldBox { text_box: self, at }
    }

    fn draw(self, f: &mut Frame, screen: Rect, theme: &ResolvedTheme, wrap: bool) -> Rect {
        let area = centered_rect_fixed(self.size.0, self.size.1, screen);
        f.render_widget(Clear, area);
        let mut paragraph = Paragraph::new(self.body)
            .block(popup_block(self.title, self.border, theme))
            .style(self.body_style);
        if wrap {
            paragraph = paragraph.wrap(Wrap { trim: true });
        }
        f.render_widget(paragraph, area);
        area
    }
}

/// A [`TextBox`] whose body holds a text field (a one-field prompt).
pub struct FieldBox<'a> {
    pub text_box: TextBox<'a>,
    /// Row and column of the field's first character inside the body.
    pub at: (u16, u16),
}

impl FieldBox<'_> {
    pub fn render(self, f: &mut Frame, screen: Rect, theme: &ResolvedTheme) {
        let area = self.text_box.draw(f, screen, theme, false);
        let inner = area.inner(ratatui::layout::Margin::new(1, 1));
        let y = inner.y + self.at.0;
        if y < inner.bottom() {
            crate::ui::hit::register(
                Rect::new(inner.x, y, inner.width, 1),
                crate::ui::hit::HitTarget::PopupField {
                    keys: Vec::new(),
                    text_x: inner.x + self.at.1,
                },
            );
        }
    }
}

/// Where [`prompt_text`] puts the field of `template`: its line and the
/// display column it starts at.
pub fn field_at(template: &str) -> (u16, u16) {
    let before = template
        .split_once("{}")
        .map_or(template, |(before, _)| before);
    let line = before.matches('\n').count();
    let head = before.rsplit('\n').next().unwrap_or_default();
    (
        line as u16,
        crate::ui::text_width::display_width(head) as u16,
    )
}

/// A one-field prompt body: `template` with its first `{}` replaced by the
/// (always focused) `field`, in the popup foreground with a reversed cursor.
pub fn prompt_text(template: &str, field: &TextField, theme: &ResolvedTheme) -> Text<'static> {
    let style = theme.popup_fg();
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

/// A [`TextField`] in its own bordered box (Git dialogs): yellow border and
/// bold text with a cursor when focused, dim border otherwise.
pub fn input_box(field: &TextField, focused: bool, theme: &ResolvedTheme) -> Paragraph<'static> {
    let (border, style) = if focused {
        (
            Color::Yellow,
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        (Color::DarkGray, theme.popup_fg())
    };
    let spans = field_spans(field, style, theme.selection(), focused);
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

/// Focused / unfocused styles for dialog rows and buttons.
#[derive(Debug, Clone, Copy)]
pub struct FocusStyles {
    pub active: Style,
    pub normal: Style,
}

impl FocusStyles {
    /// Cyan-on-black focus over the theme's popup foreground.
    pub fn from_theme(theme: &ResolvedTheme) -> Self {
        Self {
            active: Style::default().bg(Color::Cyan).fg(Color::Black),
            normal: theme.popup_fg(),
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

/// `"► "` in front of a focused row, two spaces otherwise.
fn focus_marker(focused: bool) -> &'static str {
    if focused { "► " } else { "  " }
}

/// `"► text"` when focused, `"  text"` otherwise, in the focus style.
pub fn marked_row<'a>(text: &str, focused: bool, styles: FocusStyles) -> Paragraph<'a> {
    let marker = focus_marker(focused);
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
    let marker = focus_marker(focused);
    let mut value = vec![Span::styled("   > ", style)];
    value.extend(field_spans(field, style, styles.cursor(), focused));
    Paragraph::new(vec![
        Line::from(format!("{marker}{label}")),
        Line::from(value),
    ])
    .style(style)
}

/// One line: `"► label value"`, the field with a cursor when focused.
pub fn inline_field<'a>(
    label: &str,
    field: &TextField,
    focused: bool,
    styles: FocusStyles,
) -> Paragraph<'a> {
    let style = styles.pick(focused);
    let marker = focus_marker(focused);
    let mut spans = vec![Span::styled(format!("{marker}{label} "), style)];
    spans.extend(field_spans(field, style, styles.cursor(), focused));
    Paragraph::new(Line::from(spans)).style(style)
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

/// [`labelled_field`] for form row `row`, drawn at `area` and clickable.
pub fn form_labelled_field(
    f: &mut Frame,
    area: Rect,
    (label, field): (&str, &TextField),
    row: usize,
    focus: mouse::FormFocus,
    styles: FocusStyles,
) {
    let focused = focus.focus == row;
    f.render_widget(labelled_field(label, field, focused, styles), area);
    focus.field(area, row, area.x + 5);
}

/// [`inline_field`] for form row `row`, drawn at `area` and clickable.
pub fn form_inline_field(
    f: &mut Frame,
    area: Rect,
    (label, field): (&str, &TextField),
    row: usize,
    focus: mouse::FormFocus,
    styles: FocusStyles,
) {
    let focused = focus.focus == row;
    f.render_widget(inline_field(label, field, focused, styles), area);
    let prefix = crate::ui::text_width::display_width(focus_marker(focused))
        + crate::ui::text_width::display_width(label)
        + 1;
    focus.field(area, row, area.x + prefix as u16);
}

/// [`marked_row`] for option row `row` (checkbox, cycle), drawn at `area`;
/// a click focuses it and presses Space.
pub fn form_option(
    f: &mut Frame,
    area: Rect,
    text: &str,
    row: usize,
    focus: mouse::FormFocus,
    styles: FocusStyles,
) {
    f.render_widget(marked_row(text, focus.focus == row, styles), area);
    focus.option(area, row);
}

/// [`button_bar`] of a form dialog drawn at `area`, its buttons clickable.
pub fn form_buttons(
    f: &mut Frame,
    area: Rect,
    labels: &[String],
    focus: mouse::FormFocus,
    styles: FocusStyles,
) {
    f.render_widget(
        button_bar(labels, focus.form.focused_button(focus.focus), styles),
        area,
    );
    focus.buttons(area, labels);
}

/// A full-width horizontal rule.
pub fn separator<'a>(width: u16, style: Style) -> Paragraph<'a> {
    Paragraph::new(ratatui::symbols::line::HORIZONTAL.repeat(width as usize)).style(style)
}
