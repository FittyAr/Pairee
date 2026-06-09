use crate::config::localization::t;
use crate::app::context::AppContext;
use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
};

pub fn render_fkeys(f: &mut Frame, area: Rect, context: &AppContext) {
    let theme = &context.config.theme;

    // F1 to F10 labels matching classic Norton Commander features
    let fkeys = [
        ("1", t("fkey_help")),
        ("2", t("fkey_menu")),
        ("3", t("fkey_view")),
        ("4", t("fkey_edit")),
        ("5", t("fkey_copy")),
        ("6", t("fkey_renmov")),
        ("7", t("fkey_mkdir")),
        ("8", t("fkey_delete")),
        ("9", t("fkey_pulldn")),
        ("10", t("fkey_quit")),
    ];

    // Divide the row into 10 equal columns
    let constraints = vec![Constraint::Ratio(1, 10); 10];
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(constraints)
        .split(area);

    let num_style = Style::default()
        .bg(parse_color(&theme.fkey_bg))
        .fg(parse_color(&theme.fkey_num_fg));

    let text_style = Style::default()
        .bg(parse_color("DarkGray"))
        .fg(parse_color(&theme.fkey_text_fg));

    for (i, (num, text)) in fkeys.iter().enumerate() {
        let block_area = columns[i];

        // Compose block as " 1 Help   "
        let line = Line::from(vec![
            Span::styled(format!(" {:>2}", num), num_style),
            Span::styled(format!(" {:<6}", text), text_style),
        ]);

        let paragraph = Paragraph::new(line);
        f.render_widget(paragraph, block_area);
    }
}
