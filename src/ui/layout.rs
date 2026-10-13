use crate::app::state::AppState;
use crate::app::state::popup::Menu;
use ratatui::layout::{Constraint, Direction, Layout, Rect};

pub struct AppLayout {
    pub menu_rect: Rect,
    pub main_rect: Rect,
    pub left_rect: Rect,
    pub right_rect: Rect,
    pub transfer_rect: Rect,
    pub cli_rect: Rect,
    pub fkeys_rect: Rect,
}

/// Splits the screen into segments respecting panel visibility flags.
///
/// Panel visibility rules:
/// - `both_panels_hidden` (`Ctrl+O`): the main area collapses to 0 height,
///   showing only the CLI and f-key bar, revealing the terminal scrollback below.
/// - `left_panel_visible` / `right_panel_visible`: when one panel is hidden,
///   the other expands to fill the full width (100 %).
pub fn calculate_layout(
    area: Rect,
    state: &AppState,
    config: &crate::config::AppConfig,
) -> AppLayout {
    let settings = &config.settings;
    let menu_active = matches!(
        state.dialogs.top(),
        Some(crate::app::state::PopupType::Menu(Menu { .. }))
    );
    // A menu bar that is not always shown lies over the top row while a
    // menu is open (as in Far), so opening it never moves the panels.
    let menu_height = u16::from(settings.interface_always_show_menu_bar);

    let fkeys_height = if settings.interface_show_key_bar && config.keybindings.preset == "norton" {
        1
    } else {
        0
    };

    let transfer_height = if let Some(ref ts) = state.transfer {
        let any_running = ts.engine.queue.get_all().iter().any(|j| {
            matches!(
                j.status,
                crate::fs::transfer::job::TransferJobStatus::Scanning
                    | crate::fs::transfer::job::TransferJobStatus::Transferring
                    | crate::fs::transfer::job::TransferJobStatus::Verifying
            )
        });
        if ts.view_mode == crate::app::state::TransferViewMode::Minimized && any_running {
            3
        } else {
            0
        }
    } else {
        0
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(menu_height), // Dropdown menu bar (F9)
            if state.panels.both_hidden {
                Constraint::Length(0) // Both panels hidden → 0 height
            } else {
                Constraint::Min(3) // Normal: panels take remaining space
            },
            Constraint::Length(transfer_height), // Transfer Compact Bar
            Constraint::Length(1),               // Command-line bar
            Constraint::Length(fkeys_height),    // F1–F10 shortcuts
        ])
        .split(area);

    // Determine panel width constraints based on individual visibility flags
    let (left_constraint, right_constraint) =
        match (state.panels.left_visible, state.panels.right_visible) {
            (true, true) => {
                let split = crate::config::settings::clamp_panel_split(settings.panel_split);
                (
                    Constraint::Percentage(split),
                    Constraint::Percentage(100 - split),
                )
            }
            (true, false) => (Constraint::Percentage(100), Constraint::Length(0)),
            (false, true) => (Constraint::Length(0), Constraint::Percentage(100)),
            (false, false) => (Constraint::Percentage(50), Constraint::Percentage(50)),
        };

    let panels_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([left_constraint, right_constraint])
        .split(chunks[1]);

    let menu_rect = if menu_height == 0 && menu_active {
        Rect { height: 1, ..area }
    } else {
        chunks[0]
    };

    AppLayout {
        menu_rect,
        main_rect: chunks[1],
        left_rect: panels_chunks[0],
        right_rect: panels_chunks[1],
        transfer_rect: chunks[2],
        cli_rect: chunks[3],
        fkeys_rect: chunks[4],
    }
}
