use crate::app::context::AppContext;
use crate::app::state::tabs::Tab;
use crate::app::state::{ActivePanel, AppState, PanelState};
use crate::config::AppConfig;
use crate::ui::draw_ui;
use crate::ui::tab_bar::{Cell, layout};
use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::{Terminal, backend::TestBackend};
use std::path::PathBuf;

fn cells(natural: &[usize], active: usize, total: usize) -> Vec<(usize, usize, usize)> {
    layout(natural, active, total)
        .into_iter()
        .map(|Cell { index, x, width }| (index, x, width))
        .collect()
}

#[test]
fn tabs_keep_their_width_when_they_fit() {
    assert_eq!(cells(&[6, 8], 0, 40), [(0, 0, 6), (1, 6, 8)]);
}

#[test]
fn widest_tabs_shrink_first() {
    assert_eq!(cells(&[20, 6], 1, 16), [(0, 0, 10), (1, 10, 6)]);
}

#[test]
fn a_window_around_the_active_tab_when_nothing_else_fits() {
    let got = cells(&[10; 6], 4, 9);
    assert_eq!(got.len(), 2, "{got:?}");
    assert!(got.iter().any(|c| c.0 == 4), "active tab stays visible");
    assert!(got.iter().all(|c| c.1 + c.2 <= 9));
    assert_eq!(cells(&[10], 0, 2), [(0, 0, 2)], "a lone tab is clipped");
}

fn app(always: bool) -> (AppContext, AppState) {
    let mut config = AppConfig::default();
    config.settings.always_show_tab_bar = always;
    let state = AppState::new(PathBuf::from("alpha_dir"), PathBuf::from("right_dir"));
    (AppContext::new(config), state)
}

fn add_tab(state: &mut AppState, path: &str) {
    let tab = Tab::new(PanelState::new(PathBuf::from(path)));
    state
        .panels
        .tabs_mut(ActivePanel::Left)
        .insert_after(None, tab);
}

fn row(terminal: &Terminal<TestBackend>, y: u16) -> String {
    let buffer = terminal.backend().buffer();
    (0..buffer.area.width)
        .map(|x| buffer[(x, y)].symbol())
        .collect()
}

fn draw(context: &AppContext, state: &AppState, w: u16, h: u16) -> Terminal<TestBackend> {
    let mut terminal = Terminal::new(TestBackend::new(w, h)).expect("terminal");
    state.tab_bar.clear();
    terminal.draw(|f| draw_ui(f, context, state)).expect("draw");
    terminal
}

/// Screen row of the left panel's first line (below the menu bar).
fn panel_top(context: &AppContext, state: &AppState, w: u16, h: u16) -> u16 {
    let area = ratatui::layout::Rect::new(0, 0, w, h);
    crate::ui::layout::calculate_layout(area, state, &context.config.settings)
        .left_rect
        .y
}

#[test]
fn bar_appears_with_a_second_tab_only() {
    let (context, mut state) = app(false);
    let top = panel_top(&context, &state, 80, 24);
    let terminal = draw(&context, &state, 80, 24);
    assert!(!row(&terminal, top).contains("1:alpha_dir"));
    add_tab(&mut state, "beta_dir");
    let terminal = draw(&context, &state, 80, 24);
    let bar = row(&terminal, top);
    assert!(
        bar.contains("1:alpha_dir") && bar.contains("2:beta_dir"),
        "{bar}"
    );
}

#[test]
fn bar_can_be_shown_for_a_single_tab() {
    let (context, state) = app(true);
    let top = panel_top(&context, &state, 80, 24);
    let terminal = draw(&context, &state, 80, 24);
    assert!(row(&terminal, top).contains("1:alpha_dir"));
}

#[test]
fn narrow_terminal_truncates_titles_with_an_ellipsis() {
    let (context, mut state) = app(false);
    add_tab(&mut state, "a_rather_long_folder_name");
    add_tab(&mut state, "another_long_folder_name");
    let top = panel_top(&context, &state, 30, 12);
    let terminal = draw(&context, &state, 30, 12);
    let bar = row(&terminal, top);
    assert!(bar.contains('…'), "{bar}");
    assert!(bar.contains("3:"), "the active tab stays visible: {bar}");
    // Degenerate sizes must not panic.
    draw(&context, &state, 8, 6);
    draw(&context, &state, 1, 3);
}

#[test]
fn clicks_on_the_bar_switch_and_close_tabs() {
    let (context, mut state) = app(false);
    add_tab(&mut state, "beta_dir");
    let first = state.panels.tabs(ActivePanel::Left).tabs()[0].id;
    let top = panel_top(&context, &state, 80, 24);
    draw(&context, &state, 80, 24);
    let click = |button| MouseEvent {
        kind: MouseEventKind::Down(button),
        column: 2,
        row: top,
        modifiers: KeyModifiers::empty(),
    };
    let handle = crate::app::app::tab_mouse::handle_tab_bar_mouse;
    assert!(handle(&mut state, click(MouseButton::Left), false));
    assert_eq!(state.panels.active_tab_id(ActivePanel::Left), first);
    assert!(handle(&mut state, click(MouseButton::Middle), false));
    assert_eq!(state.panels.tabs(ActivePanel::Left).count(), 1);
}
