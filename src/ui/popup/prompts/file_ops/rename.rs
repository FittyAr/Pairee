use crate::app::state::popup::RenamePrompt;
use crate::app::state::popup::forms::RENAME_FORM;
use crate::config::localization::t;
use crate::ui::popup::kit::{self, FocusStyles, mouse::FormFocus};
use crate::ui::popup::view::{DialogView, ViewCtx};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

/// Whether `target` exists in `dir`: from the listing of a panel showing
/// `dir` (no disk access while drawing, right for remote panels), else on
/// the local disk.
fn is_listed(
    state: &crate::app::state::AppState,
    dir: &std::path::Path,
    target: &std::path::Path,
) -> bool {
    use crate::app::state::ActivePanel;
    [ActivePanel::Left, ActivePanel::Right]
        .into_iter()
        .map(|side| state.panels.side(side))
        .find(|panel| panel.current_path == dir)
        .map_or_else(
            || target.exists(),
            |panel| panel.entries.iter().any(|e| e.path == target),
        )
}

impl DialogView for RenamePrompt {
    fn render(&self, f: &mut Frame, cx: &ViewCtx<'_>) {
        let (theme, size) = (cx.theme, cx.area);
        let RenamePrompt {
            input,
            original,
            src_path,
            parent_dir,
            cursor_idx,
        } = self;
        let inner = kit::dialog_frame(
            f,
            size,
            (60, 9),
            t("prompt_rename_title"),
            kit::fg(theme.ui.warning),
            theme,
        );
        let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1); 6]) // header, input, warning, sep, buttons, hint
        .split(inner);

        let styles = FocusStyles::from_theme(theme);
        f.render_widget(
            Paragraph::new(format!("{} {}", t("prompt_rename_to"), original)).style(styles.normal),
            chunks[0],
        );

        let form = FormFocus::new(RENAME_FORM, *cursor_idx);
        let view = kit::FieldView::new(input, usize::from(chunks[1].width.saturating_sub(2)));
        form.field(chunks[1], 0, view.hit(chunks[1].x + 2));
        let focused = *cursor_idx == 0;
        let style = styles.pick(focused);
        let mut field = vec![Span::styled("> ", style)];
        field.extend(view.spans(style, styles.cursor(), focused));
        f.render_widget(Paragraph::new(Line::from(field)).style(style), chunks[1]);

        // Live collision warning: only if the typed name actually differs from
        // the original AND a sibling with that name already exists.
        let collision = *input != original.as_str() && {
            let target = parent_dir.join(input.text());
            target != *src_path && is_listed(cx.state, parent_dir, &target)
        };
        if collision {
            let warn_style = Style::default()
                .fg(theme.ui.danger)
                .add_modifier(Modifier::BOLD);
            f.render_widget(
                Paragraph::new(format!("[!] {}", t("prompt_rename_collision"))).style(warn_style),
                chunks[2],
            );
        }

        f.render_widget(
            kit::separator(inner.width, kit::fg(theme.ui.warning)),
            chunks[3],
        );
        kit::form_buttons(
            f,
            chunks[4],
            &[t("btn_ok_bracket"), t("btn_cancel_bracket")],
            form,
            styles,
        );
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(
                t("prompt_rename_hint"),
                kit::fg(theme.ui.muted),
            )))
            .alignment(Alignment::Center),
            chunks[5],
        );
    }
}

#[cfg(test)]
mod tests {
    use crate::config::localization::t;
    use crate::test_harness::Harness;

    #[test]
    fn the_collision_warning_comes_from_the_panel_listing() {
        let mut h = Harness::new();
        h.write("work/left/a.txt", "x");
        h.write("work/left/b.txt", "y");
        h.reread().focus("a.txt").keys("@rename");
        h.keys("End Backspace*5").text("b.txt");
        h.assert_screen(&t("prompt_rename_collision"));
        h.keys("Backspace").text("x");
        h.assert_no_screen(&t("prompt_rename_collision"));
    }
}
