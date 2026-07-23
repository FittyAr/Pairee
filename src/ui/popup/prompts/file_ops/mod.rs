pub mod compress;
pub mod copy;
pub mod delete;
pub mod describe;
pub mod link;
pub mod mkdir;
pub mod rename;
pub mod rename_move;
pub mod wipe;

use crate::app::state::PopupType;
use ratatui::{Frame, layout::Rect};

pub fn render(
    f: &mut Frame,
    popup: &PopupType,
    theme: &crate::config::theme::Theme,
    size: Rect,
) -> bool {
    match popup {
        PopupType::MkDirPrompt { .. } => mkdir::render(f, popup, theme, size),
        PopupType::CopyPrompt { .. } => copy::render(f, popup, theme, size),
        PopupType::MovePrompt { .. } => rename_move::render(f, popup, theme, size),
        PopupType::RenamePrompt { .. } => rename::render(f, popup, theme, size),
        PopupType::ConfirmDelete { .. } => delete::render(f, popup, theme, size),
        PopupType::WipeConfirm { .. } => wipe::render(f, popup, theme, size),
        PopupType::CreateLinkPrompt { .. } => link::render(f, popup, theme, size),
        PopupType::DescribeFilePrompt { .. } => describe::render(f, popup, theme, size),
        PopupType::CompressPrompt { .. } => compress::render(f, popup, theme, size),
        _ => false,
    }
}
