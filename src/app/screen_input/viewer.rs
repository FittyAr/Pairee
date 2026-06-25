use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType, Screen};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub fn handle_viewer_screen(
    state: &mut AppState,
    key: KeyEvent,
    _context: &mut AppContext,
) -> Result<(), ()> {
    let is_ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

    // Global pass through
    if key.code == KeyCode::F(12) || (key.code == KeyCode::Tab && is_ctrl) {
        return Err(());
    }

    if let Some(Screen::Viewer(vw)) = state.screens.get_mut(state.active_screen_idx) {
        match key.code {
            KeyCode::Up => vw.scroll_up(1),
            KeyCode::Down => vw.scroll_down(1),
            KeyCode::PageUp => vw.scroll_up(20),
            KeyCode::PageDown => vw.scroll_down(20),
            KeyCode::Home => vw.scroll = 0,
            KeyCode::End => {
                if vw.mode == crate::ui::viewer::ViewerMode::Text {
                    vw.scroll = vw.lines.len().saturating_sub(1);
                } else if vw.mode == crate::ui::viewer::ViewerMode::Hex {
                    vw.scroll = (vw.raw.len() / 16).saturating_sub(1);
                } else {
                    if let Some(ref img) = vw.image_data {
                        vw.scroll = (img.height() as usize / 2).saturating_sub(1);
                    }
                }
            }
            KeyCode::F(4) => vw.toggle_mode(),
            KeyCode::F(7) => {
                state.active_popup = Some(PopupType::ViewerSearchPrompt {
                    query: String::new(),
                    case_sensitive: false,
                    cursor_idx: 0,
                });
                return Ok(());
            }
            KeyCode::F(3) => {
                if let Some(ref q) = vw.last_search {
                    if vw.mode == crate::ui::viewer::ViewerMode::Text {
                        let cs = vw.last_case_sensitive;
                        let match_fn = |l: &str| {
                            if cs {
                                l.contains(q)
                            } else {
                                l.to_lowercase().contains(&q.to_lowercase())
                            }
                        };
                        // basic search downward from current scroll
                        if let Some(found_idx) = vw
                            .lines
                            .iter()
                            .enumerate()
                            .skip(vw.scroll + 1)
                            .find(|(_, l)| match_fn(l))
                            .map(|(i, _)| i)
                        {
                            vw.scroll = found_idx;
                        } else if let Some(found_idx) = vw
                            .lines
                            .iter()
                            .enumerate()
                            .take(vw.scroll + 1)
                            .find(|(_, l)| match_fn(l))
                            .map(|(i, _)| i)
                        {
                            vw.scroll = found_idx;
                        }
                    }
                }
            }
            KeyCode::Esc | KeyCode::F(10) => {
                state.close_current_screen();
                return Ok(());
            }
            _ => {}
        }
        return Ok(());
    }
    Err(())
}
