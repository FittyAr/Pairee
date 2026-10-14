use crate::app::state::popup::PluginMenuState;
use crate::config::localization::t;
use crate::ui::popup::view::{DialogView, ViewCtx};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

mod detail;
pub mod dev;
pub mod installed;
pub mod search;
pub mod select;
pub mod settings;

pub use crate::ui::wrap::wrap_text;

/// Where a plugin-manager tab draws: list and detail areas plus styles.
pub struct Pane<'a> {
    pub list_area: Rect,
    pub detail_area: Rect,
    pub theme: &'a crate::ui::resolved_theme::ResolvedTheme,
    pub border_style: Style,
    pub bg_style: Style,
}

/// Rotating spinner character for indeterminate progress (~5 fps).
pub(crate) fn spinner_frame() -> &'static str {
    const FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() / 200)
        .unwrap_or(0);
    FRAMES[(now as usize) % FRAMES.len()]
}

/// Whether the current tab shows a one-line input above the list.
fn has_input_row(menu: &crate::app::state::PluginMenuState) -> bool {
    menu.active_tab == 1 || (menu.active_tab == 2 && menu.editing_query)
}

/// The tab bar: Installed, Search and (in developer mode) Dev.
fn tabs_line(
    active_tab: usize,
    dev_mode: bool,
    theme: &crate::ui::resolved_theme::ResolvedTheme,
) -> Line<'static> {
    let dim = Style::default().fg(theme.ui.muted);
    let tab_style = |idx: usize| {
        if active_tab == idx {
            Style::default()
                .fg(theme.ui.warning)
                .add_modifier(Modifier::BOLD)
        } else {
            dim
        }
    };
    let mut tabs = vec![("plugin_tab_installed", 0), ("plugin_tab_search", 1)];
    if dev_mode {
        tabs.push(("plugin_tab_dev", 2));
    }
    let mut spans = Vec::new();
    for (pos, (key, idx)) in tabs.into_iter().enumerate() {
        spans.push(Span::styled(if pos == 0 { " [ " } else { "  [ " }, dim));
        spans.push(Span::styled(t(key), tab_style(idx)));
        spans.push(Span::styled(" ]", dim));
    }
    Line::from(spans)
}

/// A click on a tab presses Tab (which only walks forward) until it is open.
fn register_tabs(line: &Line, area: Rect, active: usize, count: usize) {
    use crate::ui::popup::kit::mouse;
    // Each tab is three spans: opening bracket, name, closing bracket.
    let labels: Vec<String> = line
        .spans
        .chunks(3)
        .map(|tab| tab.iter().map(|s| s.content.as_ref()).collect())
        .collect();
    let row = area.inner(ratatui::layout::Margin::new(1, 1));
    let keys = (0..labels.len())
        .map(|tab| {
            let forward = (tab + count - active) % count;
            mouse::presses(
                forward as i64,
                crossterm::event::KeyCode::Tab,
                crossterm::event::KeyCode::Tab,
            )
        })
        .collect();
    mouse::span_buttons(row, &labels, (0, false), keys);
}

/// Text and title of the input row: the registry query on the Search tab,
/// or the current developer-wizard prompt.
fn input_row(menu: &crate::app::state::PluginMenuState) -> (String, String) {
    if menu.active_tab == 1 {
        return (
            format!("{}{}|", t("plugin_query"), menu.search_query),
            t("plugin_search_repo"),
        );
    }
    let label = match menu.dev_wizard_step {
        2 => "plugin_enter_desc",
        3 => "plugin_enter_author",
        5 => "plugin_enter_commit_desc",
        6 => "plugin_enter_token_optional",
        10 => "plugin_enter_active_dev",
        _ => "plugin_enter_name",
    };
    let title = match menu.dev_wizard_step {
        5 | 6 => t("plugin_dev_opt_submit"),
        10 => t("plugin_dev_opt_active").replace("{}", ""),
        _ => t("plugin_init_title"),
    };
    (format!("{}{}|", t(label), menu.search_query), title)
}

impl DialogView for PluginMenuState {
    fn render(&self, f: &mut Frame, cx: &ViewCtx<'_>) {
        let (theme, size, context) = (cx.theme, cx.area, cx.context);
        let menu = self;
        let area = super::centered_rect_in(85, 80, size);
        f.render_widget(Clear, area);

        let border_style = Style::default().fg(theme.popup_border);
        let bg_style = Style::default().bg(theme.popup_bg);
        f.render_widget(Block::default().style(bg_style), area);

        let input = has_input_row(menu);
        let mut constraints = vec![Constraint::Length(3)]; // tab bar
        if input {
            constraints.push(Constraint::Length(3)); // search input / prompt
        }
        constraints.extend([Constraint::Min(1), Constraint::Length(1)]); // content, legend
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(area);
        let (content_area, legend_area) = (chunks[chunks.len() - 2], chunks[chunks.len() - 1]);

        let tab_block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(t("plugin_manager_title"))
            .style(bg_style);
        let dev_mode = context.config.settings.plugins_developer_mode;
        let tabs = tabs_line(menu.active_tab, dev_mode, theme);
        register_tabs(
            &tabs,
            chunks[0],
            menu.active_tab,
            if dev_mode { 3 } else { 2 },
        );
        f.render_widget(Paragraph::new(tabs).block(tab_block), chunks[0]);

        if input {
            let (text, title) = input_row(menu);
            let input_border = if menu.editing_query {
                theme.ui.warning
            } else {
                theme.popup_border
            };
            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(input_border))
                .title(title)
                .style(bg_style);
            f.render_widget(Paragraph::new(text).block(block), chunks[1]);
        }

        let content_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(content_area);
        let pane = Pane {
            list_area: content_chunks[0],
            detail_area: content_chunks[1],
            theme,
            border_style,
            bg_style,
        };

        let hint_key = match menu.active_tab {
            0 => {
                installed::render_installed(f, &pane, menu);
                "plugin_hint_tab0"
            }
            1 => {
                search::render_search(f, &pane, menu);
                "plugin_hint_tab1"
            }
            _ => {
                dev::render_dev(f, &pane, menu, &context.config.settings.active_dev_plugin);
                "plugin_hint_tab2"
            }
        };
        f.render_widget(
            Paragraph::new(t(hint_key))
                .alignment(ratatui::layout::Alignment::Center)
                .style(Style::default().fg(theme.ui.muted)),
            legend_area,
        );
    }
}
