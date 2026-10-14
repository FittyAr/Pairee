use crate::app::state::popup::CreateKind;
use crate::app::state::popup::MkDirPrompt;
use crate::app::state::popup::forms::MKDIR_FORM;
use crate::config::localization::t;
use crate::ui::popup::kit::{self, FocusStyles, mouse::FormFocus};
use crate::ui::popup::view::{DialogView, ViewCtx};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    text::Line,
    widgets::Paragraph,
};

impl DialogView for MkDirPrompt {
    fn render(&self, f: &mut Frame, cx: &ViewCtx<'_>) {
        let (theme, size) = (cx.theme, cx.area);
        let MkDirPrompt {
            input,
            cursor_idx,
            process_multiple,
            kind,
        } = self;
        let inner = kit::dialog_frame(
            f,
            size,
            (50, 9),
            t(match kind {
                CreateKind::Folder => "prompt_mkdir_title",
                CreateKind::File => "prompt_new_file_title",
                CreateKind::Auto => "prompt_create_title",
            }),
            kit::fg(theme.popup_border),
            theme,
        );
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(2),
                Constraint::Length(2),
            ])
            .split(inner);

        let styles = FocusStyles::from_theme(theme);
        let form = FormFocus::new(MKDIR_FORM, *cursor_idx);
        let field_row = Rect {
            y: chunks[0].y + 1,
            height: 1,
            ..chunks[0]
        };
        let view = kit::FieldView::new(input, usize::from(field_row.width.saturating_sub(3)));
        form.field(field_row, 0, view.hit(field_row.x + 3));
        form.option(chunks[1], 1);
        let focused = *cursor_idx == 0;
        let style = styles.pick(focused);
        let mut field = vec![ratatui::text::Span::styled(" > ", style)];
        field.extend(view.spans(style, styles.cursor(), focused));
        f.render_widget(
            Paragraph::new(vec![
                Line::from(t(match kind {
                    CreateKind::Folder => "prompt_mkdir_to",
                    CreateKind::File => "prompt_new_file_to",
                    CreateKind::Auto => "prompt_create_to",
                })),
                Line::from(field),
            ])
            .style(style),
            chunks[0],
        );
        f.render_widget(
            Paragraph::new(kit::checkbox_row(
                *process_multiple,
                &t("prompt_process_multiple_names"),
            ))
            .style(styles.row(*cursor_idx, 1)),
            chunks[1],
        );
        kit::form_buttons(
            f,
            chunks[2],
            &[t("btn_ok_bracket"), t("btn_cancel_bracket")],
            form,
            styles,
        );
    }
}
