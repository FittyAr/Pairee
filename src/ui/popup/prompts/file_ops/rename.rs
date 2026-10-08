use crate::app::state::PopupType;
use crate::app::state::popup::forms::RENAME_FORM;
use crate::config::localization::t;
use crate::ui::popup::kit::{self, FocusStyles};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

pub fn render(
    f: &mut Frame,
    popup: &PopupType,
    theme: &crate::config::theme::Theme,
    size: Rect,
) -> bool {
    let PopupType::RenamePrompt {
        input,
        original,
        src_path,
        parent_dir,
        cursor_idx,
    } = popup
    else {
        return false;
    };
    let inner = kit::dialog_frame(
        f,
        size,
        (60, 9),
        t("prompt_rename_title"),
        kit::fg(Color::Yellow),
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

    let focused = *cursor_idx == 0;
    let style = styles.pick(focused);
    let mut field = vec![Span::styled("> ", style)];
    field.extend(kit::field_spans(input, style, styles.cursor(), focused));
    f.render_widget(Paragraph::new(Line::from(field)).style(style), chunks[1]);

    // Live collision warning: only if the typed name actually differs from
    // the original AND a sibling with that name already exists.
    let collision = *input != original.as_str() && {
        let target = parent_dir.join(input.text());
        target != *src_path && target.exists()
    };
    if collision {
        let warn_style = Style::default().fg(Color::Red).add_modifier(Modifier::BOLD);
        f.render_widget(
            Paragraph::new(format!("[!] {}", t("prompt_rename_collision"))).style(warn_style),
            chunks[2],
        );
    }

    f.render_widget(
        kit::separator(inner.width, kit::fg(Color::Yellow)),
        chunks[3],
    );
    f.render_widget(
        kit::button_bar(
            &[t("btn_ok_bracket"), t("btn_cancel_bracket")],
            RENAME_FORM.focused_button(*cursor_idx),
            styles,
        ),
        chunks[4],
    );
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            t("prompt_rename_hint"),
            kit::fg(Color::DarkGray),
        )))
        .alignment(Alignment::Center),
        chunks[5],
    );
    true
}
