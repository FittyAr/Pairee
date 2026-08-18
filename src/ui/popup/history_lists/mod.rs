mod associations;
mod compare;
mod lists;
mod search;
mod task_list;
mod tree;

use crate::app::state::PopupType;
use crate::ui::scrollbar::ScrollbarUiState;
use ratatui::{Frame, layout::Rect};

pub fn render_history_lists_popup(
    f: &mut Frame,
    popup: &PopupType,
    theme: &crate::config::theme::Theme,
    size: Rect,
    scrollbar: Option<&ScrollbarUiState>,
) -> bool {
    lists::render_history_lists(f, popup, theme, size, scrollbar)
        || search::render_search(f, popup, theme, size, scrollbar)
        || tree::render_tree(f, popup, theme, size, scrollbar)
        || compare::render_compare(f, popup, theme, size, scrollbar)
        || task_list::render_task_list(f, popup, theme, size, scrollbar)
        || associations::render_associations(f, popup, theme, size, scrollbar)
}
