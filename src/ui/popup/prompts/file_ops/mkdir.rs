use crate::app::state::PopupType;
use crate::app::state::popup::CreateKind;
use crate::app::state::popup::forms::MKDIR_FORM;
use crate::config::localization::t;
use crate::ui::popup::kit::{self, FocusStyles};
use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    text::Line,
    widgets::Paragraph,
};

pub fn render(
    f: &mut Frame,
    popup: &PopupType,
    theme: &crate::config::theme::Theme,
    size: Rect,
) -> bool {
    let PopupType::MkDirPrompt {
        input,
        cursor_idx,
        process_multiple,
        kind,
    } = popup
    else {
        return false;
    };
    let inner = kit::dialog_frame(
        f,
        size,
        (50, 9),
        t(match kind {
            CreateKind::Folder => "prompt_mkdir_title",
            CreateKind::File => "prompt_new_file_title",
            CreateKind::Auto => "prompt_create_title",
        }),
        kit::fg(parse_color(&theme.popup_border)),
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
    let focused = *cursor_idx == 0;
    let style = styles.pick(focused);
    let mut field = vec![ratatui::text::Span::styled(" > ", style)];
    field.extend(kit::field_spans(input, style, styles.cursor(), focused));
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
    f.render_widget(
        kit::button_bar(
            &[t("btn_ok_bracket"), t("btn_cancel_bracket")],
            MKDIR_FORM.focused_button(*cursor_idx),
            styles,
        ),
        chunks[2],
    );
    true
}
