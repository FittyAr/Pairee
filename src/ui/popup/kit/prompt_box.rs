//! One-field prompts: a centered box whose text holds a text field.

use super::{FieldView, TextBox, template_with_field};
use crate::app::text_input::TextField;
use crate::ui::popup::centered_area;
use crate::ui::resolved_theme::ResolvedTheme;
use crate::ui::text_width::display_width;
use ratatui::{
    Frame,
    layout::{Margin, Rect},
    style::{Modifier, Style},
};

/// A [`TextBox`] whose body is `template` (a localized, possibly
/// multi-line text) with its first `{}` replaced by the always focused
/// `field`. The field scrolls sideways within the room left on its line,
/// and a click on that line puts its cursor under the pointer.
pub struct PromptBox<'a> {
    pub size: (u16, u16),
    pub title: String,
    pub border: Style,
    pub template: &'a str,
    pub field: &'a TextField,
}

impl PromptBox<'_> {
    pub fn render(self, f: &mut Frame, screen: Rect, theme: &ResolvedTheme) {
        // Where `TextBox` puts it (it records the dialog for the mouse).
        let area =
            crate::ui::placement::place(centered_area(self.size.0, self.size.1, screen), screen);
        let inner = area.inner(Margin::new(1, 1));
        let (line, column, tail) = field_at(self.template);
        let room = usize::from(inner.width).saturating_sub(column + display_width(tail));
        let view = FieldView::new(self.field, room);
        let style = theme.popup_fg();
        TextBox {
            size: self.size,
            title: self.title,
            border: self.border,
            body: template_with_field(
                self.template,
                &view,
                style,
                style.add_modifier(Modifier::REVERSED),
                true,
            ),
            body_style: style,
        }
        .render(f, screen, theme);
        let y = inner.y + u16::try_from(line).unwrap_or(u16::MAX);
        if y < inner.bottom() {
            let text_x = inner.x + u16::try_from(column).unwrap_or(u16::MAX);
            crate::ui::hit::register(
                Rect::new(inner.x, y, inner.width, 1),
                crate::ui::hit::HitTarget::PopupField {
                    keys: Vec::new(),
                    field: view.hit(text_x),
                },
            );
        }
    }
}

/// Where the field of `template` goes: its line, the display column it
/// starts at and the rest of that line after it.
fn field_at(template: &str) -> (usize, usize, &str) {
    let (before, after) = template.split_once("{}").unwrap_or((template, ""));
    let line = before.matches('\n').count();
    let head = before.rsplit('\n').next().unwrap_or_default();
    let tail = after.split('\n').next().unwrap_or_default();
    (line, display_width(head), tail)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_field_sits_after_the_text_before_it() {
        assert_eq!(field_at("\n Name:\n > {}.zip\n hint"), (2, 3, ".zip"));
        assert_eq!(field_at("{}"), (0, 0, ""));
    }

    #[test]
    fn a_long_field_keeps_its_cursor_and_the_text_after_it_in_the_box() {
        let theme = ResolvedTheme::default();
        let field = TextField::new(format!("{}END", "x".repeat(200)));
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(40, 7)).unwrap();
        terminal
            .draw(|f| {
                PromptBox {
                    size: (30, 5),
                    title: "t".into(),
                    border: Style::default(),
                    template: "\n > {}.zip",
                    field: &field,
                }
                .render(f, f.area(), &theme)
            })
            .unwrap();
        let row: String = (0..40)
            .map(|x| terminal.backend().buffer()[(x, 3)].symbol().to_string())
            .collect();
        assert!(row.contains("xEND .zip"), "{row:?}");
    }
}
