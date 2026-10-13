//! The plugin settings form: a checkbox or text row per setting, the
//! focused setting's description and default, the error of the last Save,
//! the Save / Defaults / Cancel bar and a key hint.

use crate::app::state::popup::{PluginSettingRow, PluginSettingValue, PluginSettingsDialog};
use crate::config::localization::t;
use crate::ui::popup::kit::{self, FocusStyles, mouse::FormFocus};
use crate::ui::popup::view::{DialogView, ViewCtx};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    widgets::{Paragraph, Wrap},
};

/// Lines under the rows: blank, description (2), error, buttons, hint.
const FOOTER_LINES: u16 = 6;
const WIDTH: u16 = 72;

/// First row shown so that `focus` is visible in `visible` rows.
fn first_row(focus: usize, rows: usize, visible: usize) -> usize {
    let focus = focus.min(rows.saturating_sub(1));
    focus.saturating_sub(visible.saturating_sub(1))
}

/// Description and default of `row`.
fn help_text(row: &PluginSettingRow) -> String {
    let mut text = row.spec.description.clone().unwrap_or_default();
    if let Some(default) = &row.spec.default {
        if !text.is_empty() {
            text.push_str("  ");
        }
        text.push_str(&t("plugin_settings_default").replacen("{}", default, 1));
    }
    text
}

fn render_row(
    f: &mut Frame,
    area: Rect,
    index: usize,
    dialog: &PluginSettingsDialog,
    styles: FocusStyles,
) {
    let row = &dialog.rows[index];
    let form = FormFocus::new(dialog.layout(), dialog.focus);
    match &row.value {
        PluginSettingValue::Toggle(on) => {
            let text = kit::checkbox_row(*on, &row.spec.key);
            kit::form_option(f, area, &text, index, form, styles);
        }
        PluginSettingValue::Text(field) => {
            let label = format!("{}:", row.spec.key);
            kit::form_inline_field(f, area, (&label, field), index, form, styles);
        }
    }
}

impl DialogView for PluginSettingsDialog {
    fn render(&self, f: &mut Frame, cx: &ViewCtx<'_>) {
        let (theme, size) = (cx.theme, cx.area);
        let max_rows = size.height.saturating_sub(FOOTER_LINES + 4).max(1) as usize;
        let visible = self.rows.len().min(max_rows);
        let height = visible as u16 + FOOTER_LINES + 2;
        let inner = kit::dialog_frame(
            f,
            size,
            (WIDTH.min(size.width.saturating_sub(2)), height),
            t("plugin_settings_title").replacen("{}", &self.plugin, 1),
            kit::fg(theme.popup_border),
            theme,
        );
        let styles = FocusStyles::from_theme(theme);
        let line = |offset: u16, lines: u16| Rect {
            y: inner.y + offset,
            height: lines,
            ..inner
        };
        let first = first_row(self.focus, self.rows.len(), visible);
        for (slot, index) in (first..self.rows.len()).take(visible).enumerate() {
            render_row(f, line(slot as u16, 1), index, self, styles);
        }
        let below = visible as u16 + 1;
        if let Some(row) = self.rows.get(self.focus) {
            f.render_widget(
                Paragraph::new(help_text(row))
                    .style(kit::fg(Color::DarkGray))
                    .wrap(Wrap { trim: true }),
                line(below, 2),
            );
        }
        if let Some((_, message)) = &self.error {
            f.render_widget(
                Paragraph::new(message.as_str()).style(Style::default().fg(Color::Red)),
                line(below + 2, 1),
            );
        }
        kit::form_buttons(
            f,
            line(below + 3, 1),
            &[
                t("plugin_settings_save"),
                t("plugin_settings_defaults"),
                t("btn_cancel_bracket"),
            ],
            FormFocus::new(self.layout(), self.focus),
            styles,
        );
        f.render_widget(kit::hint(t("plugin_settings_hint")), line(below + 4, 1));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_focused_row_stays_visible() {
        assert_eq!(first_row(0, 10, 4), 0);
        assert_eq!(first_row(3, 10, 4), 0);
        assert_eq!(first_row(6, 10, 4), 3);
        // Focus on a button: the last rows stay shown.
        assert_eq!(first_row(11, 10, 4), 6);
    }
}
