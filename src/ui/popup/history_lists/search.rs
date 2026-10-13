use crate::app::input_popup::search::SEARCH_FORM;
use crate::app::state::popup::SearchPrompt;
use crate::app::state::popup::SearchResults;
use crate::config::localization::t;
use crate::ui::popup::centered_rect_in;
use crate::ui::popup::kit;
use crate::ui::popup::view::{DialogView, ViewCtx};
use ratatui::{
    Frame,
    style::{Color, Style},
    widgets::{Block, Borders, Clear, Paragraph},
};

/// The search form: root folder, name and content queries, options and buttons.
impl DialogView for SearchPrompt {
    fn render(&self, f: &mut Frame, cx: &ViewCtx<'_>) {
        use ratatui::layout::{Constraint, Direction, Layout};
        let (theme, size) = (cx.theme, cx.area);
        let SearchPrompt {
            query,
            content_query,
            search_root,
            case_sensitive,
            search_target,
            cursor_idx,
        } = self;

        let area = centered_rect_in(65, 40, size);
        f.render_widget(Clear, area);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .title(t("prompt_search_title"))
            .style(Style::default().bg(theme.popup_bg));

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
        let focus = kit::mouse::FormFocus::new(SEARCH_FORM, *cursor_idx);
        f.render_widget(
            Paragraph::new(format!(
                " {}: {}",
                t("prompt_find_folder"),
                search_root.to_string_lossy()
            ))
            .style(styles.normal),
            chunks[0],
        );
        kit::form_labelled_field(
            f,
            chunks[1],
            (&t("prompt_find_pattern"), query),
            0,
            focus,
            styles,
        );
        kit::form_labelled_field(
            f,
            chunks[2],
            (&t("prompt_find_content"), content_query),
            1,
            focus,
            styles,
        );
        f.render_widget(kit::separator(inner.width, kit::fg(Color::Cyan)), chunks[3]);
        kit::form_option(
            f,
            chunks[4],
            &kit::checkbox_row(*case_sensitive, &t("sys_case_sensitive")),
            2,
            focus,
            styles,
        );
        kit::form_option(
            f,
            chunks[5],
            &format!(
                "{} < {} >",
                t("search_target_label"),
                t(search_target.label_key())
            ),
            3,
            focus,
            styles,
        );
        kit::form_buttons(
            f,
            chunks[7],
            &[t("btn_ok_bracket"), t("btn_cancel_bracket")],
            focus,
            styles,
        );
    }
}

/// Found files and folders, filled while the search runs.
impl DialogView for SearchResults {
    fn render(&self, f: &mut Frame, cx: &ViewCtx<'_>) {
        let (theme, size) = (cx.theme, cx.area);
        let SearchResults {
            query,
            results,
            cursor_idx,
            searching,
        } = self;
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
                    ("📄", theme.popup_fg())
                };
                (format!(" {}  {} ", icon, path.to_string_lossy()), style)
            })
            .collect();
        kit::ListPopup {
            area: centered_rect_in(70, 60, size),
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
    }
}
