use ratatui::layout::{Constraint, Direction, Layout, Rect};

#[allow(dead_code)]
pub struct AppLayout {
    pub menu_rect: Rect,
    pub main_rect: Rect,
    pub left_rect: Rect,
    pub right_rect: Rect,
    pub cli_rect: Rect,
    pub fkeys_rect: Rect,
}

/// Splits the screen into segments: top bar, main directories region, command prompt, and functional keys.
pub fn calculate_layout(area: Rect) -> AppLayout {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Dropdown menu (F9)
            Constraint::Min(3),    // Middle panels
            Constraint::Length(1), // Command command-line block
            Constraint::Length(1), // F1-F10 keys shortcuts
        ])
        .split(area);

    let panels_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50), // Left Panel
            Constraint::Percentage(50), // Right Panel
        ])
        .split(chunks[1]);

    AppLayout {
        menu_rect: chunks[0],
        main_rect: chunks[1],
        left_rect: panels_chunks[0],
        right_rect: panels_chunks[1],
        cli_rect: chunks[2],
        fkeys_rect: chunks[3],
    }
}
