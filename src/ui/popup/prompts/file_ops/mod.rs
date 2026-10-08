pub mod compress;
pub mod delete;
pub mod describe;
pub mod link;
pub mod mkdir;
pub mod multi_rename;
pub mod rename;
pub mod transfer;
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
        PopupType::TransferPrompt(prompt) => transfer::render(f, prompt, theme, size),
        PopupType::RenamePrompt { .. } => rename::render(f, popup, theme, size),
        PopupType::MultiRename(dialog) => multi_rename::render(f, dialog, theme, size),
        PopupType::ConfirmDelete { .. } => delete::render(f, popup, theme, size),
        PopupType::WipeConfirm { .. } => wipe::render(f, popup, theme, size),
        PopupType::CreateLinkPrompt { .. } => link::render(f, popup, theme, size),
        PopupType::DescribeFilePrompt { .. } => describe::render(f, popup, theme, size),
        PopupType::CompressPrompt { .. } => compress::render(f, popup, theme, size),
        _ => false,
    }
}
