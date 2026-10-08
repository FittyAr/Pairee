use crate::app::input_popup::search::SEARCH_FORM;
use crate::app::state::PopupType;
use crate::config::localization::t;
use crate::ui::popup::centered_rect;
use crate::ui::popup::kit;
use crate::ui::scrollbar::ScrollbarUiState;
use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, Clear, Paragraph},
};

pub(super) fn render_search(
    f: &mut Frame,
    popup: &PopupType,
    theme: &crate::config::theme::Theme,
    size: Rect,
    _scrollbar: Option<&ScrollbarUiState>,
) -> bool {
    match popup {
        PopupType::SearchPrompt {
            query,
            content_query,
            search_root,
            case_sensitive,
            search_target,
            cursor_idx,
        } => {
            use ratatui::layout::{Constraint, Direction, Layout};

            let area = centered_rect(65, 40, size);
            f.render_widget(Clear, area);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(t("prompt_search_title"))
                .style(Style::default().bg(parse_color(&theme.popup_bg)));

            let inner = block.inner(area);
            f.render_widget(block, area);

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1), // 0: Search in root folder
                    Constraint::Length(2), // 1: File name pattern
                    Constraint::Length(2), // 2: Content query
                    Constraint::Length(1), // 3: Separator line
                    Constraint::Length(1), // 4: Case sensitive
                    Constraint::Length(1), // 5: Search target
                    Constraint::Min(1),    // 6: Spacer
                    Constraint::Length(1), // 7: Buttons
                ])
                .split(inner);

            let styles = kit::FocusStyles::from_theme(theme);
            let focus = *cursor_idx;
            f.render_widget(
                Paragraph::new(format!(
                    " {}: {}",
                    t("prompt_find_folder"),
                    search_root.to_string_lossy()
                ))
                .style(styles.normal),
                chunks[0],
            );
            f.render_widget(
                kit::labelled_field(&t("prompt_find_pattern"), query, focus == 0, styles),
                chunks[1],
            );
            f.render_widget(
                kit::labelled_field(&t("prompt_find_content"), content_query, focus == 1, styles),
                chunks[2],
            );
            f.render_widget(kit::separator(inner.width, kit::fg(Color::Cyan)), chunks[3]);
            f.render_widget(
                kit::marked_row(
                    &kit::checkbox_row(*case_sensitive, &t("sys_case_sensitive")),
                    focus == 2,
                    styles,
                ),
                chunks[4],
            );
            f.render_widget(
                kit::marked_row(
                    &format!(
                        "{} < {} >",
                        t("search_target_label"),
                        t(search_target.label_key())
                    ),
                    focus == 3,
                    styles,
                ),
                chunks[5],
            );
            f.render_widget(
                kit::button_bar(
                    &[t("btn_ok_bracket"), t("btn_cancel_bracket")],
                    SEARCH_FORM.focused_button(focus),
                    styles,
                ),
                chunks[7],
            );

            true
        }
        PopupType::SearchResults {
            query,
            results,
            cursor_idx,
            searching,
        } => {
            let mut title = t("search_results_title").replacen("{}", query, 1).replacen(
                "{}",
                &results.len().to_string(),
                1,
            );
            if *searching {
                title.push_str(&t("searching_suffix"));
            }
            let rows = results
                .iter()
                .map(|(path, is_dir)| {
                    let (icon, style) = if *is_dir {
                        ("📁", Style::default().fg(Color::LightBlue))
                    } else {
                        ("📄", kit::popup_fg(theme))
                    };
                    (format!(" {}  {} ", icon, path.to_string_lossy()), style)
                })
                .collect();
            kit::ListPopup {
                area: centered_rect(70, 60, size),
                title,
                border: Color::Cyan,
                empty: Some(t(if *searching {
                    "searching_placeholder"
                } else {
                    "search_results_empty"
                })),
                header: Vec::new(),
                rows,
                cursor: *cursor_idx,
                scroll: kit::Scroll::HalfPage,
                hint: Some(t("search_results_hint")),
                scrollbar: None,
            }
            .render(f, theme);
            true
        }
        _ => false,
    }
}
