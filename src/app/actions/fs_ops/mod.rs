pub mod apply;
pub mod archive_cmd;
pub mod attributes;
pub mod capability;
pub mod compress;
pub mod copy_path;
pub mod delete;
pub mod describe;
pub mod edit;
pub mod extract;
pub mod folder_size;
pub mod helper;
pub mod link;
pub mod mkdir;
pub mod multi_rename;
pub mod rename;
pub mod transfer;
pub mod view;
pub mod wipe;

use crate::app::context::AppContext;
use crate::app::state::AppState;
use crate::app::state::popup::TransferPromptOp;
use crate::keybindings::Action;
use crate::terminal::TerminalBackend;

pub fn handle_fs_action(
    state: &mut AppState,
    action: &Action,
    context: &mut AppContext,
    terminal_backend: &mut TerminalBackend,
) -> bool {
    if capability::refuse_unsupported(state, action) {
        return true;
    }
    match action {
        Action::View | Action::ViewAlt => view::handle(state, action, context, terminal_backend),
        Action::Edit => edit::handle(state, context),
        Action::Copy => transfer::handle(state, context, TransferPromptOp::Copy),
        Action::CopyPath => copy_path::handle(state),
        Action::Move => transfer::handle(state, context, TransferPromptOp::Move),
        Action::Rename => rename::handle(state, context),
        Action::MultiRename => multi_rename::handle(state),
        Action::CompressFiles => compress::handle(state, context),
        Action::ExtractArchive => extract::handle(state),
        Action::MkDir => mkdir::handle(state),
        Action::Delete => delete::handle(state, context),
        Action::WipeFile => wipe::handle(state, context),
        Action::CreateLink => link::handle(state),
        Action::FileAttributes => attributes::handle(state),
        Action::ApplyCommand => apply::handle(state),
        Action::DescribeFile => describe::handle(state),
        Action::ArchiveCommands => archive_cmd::handle(state),
        Action::CalculateFolderSizes => folder_size::calculate(state),
        Action::DiskUsage => folder_size::open_disk_usage(state),
        _ => false,
    }
}
