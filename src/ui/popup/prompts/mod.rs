pub mod confirm;
pub mod file_ops;
pub mod filter;
pub mod help;
pub mod plugin;
pub mod progress;
pub mod ssh;
pub mod system;

use crate::app::state::PopupType;
use ratatui::{Frame, layout::Rect};

pub fn render_prompt_popup(
    f: &mut Frame,
    popup: &PopupType,
    theme: &crate::ui::resolved_theme::ResolvedTheme,
    size: Rect,
) -> bool {
    match popup {
        PopupType::Plugin(_) => plugin::render(f, popup, theme, size),

        _ => false,
    }
}
