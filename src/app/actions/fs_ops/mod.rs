pub mod apply;
pub mod archive_cmd;
pub mod attributes;
pub mod compress;
pub mod copy;
pub mod delete;
pub mod describe;
pub mod edit;
pub mod extract;
pub mod helper;
pub mod link;
pub mod mkdir;
pub mod move_rename;
pub mod view;
pub mod wipe;

use crate::app::context::AppContext;
use crate::app::state::AppState;
use crate::keybindings::Action;
use crate::terminal::TerminalBackend;

pub fn handle_fs_action(
    state: &mut AppState,
    action: &Action,
    context: &mut AppContext,
    terminal_backend: &mut TerminalBackend,
) -> bool {
    match action {
        Action::View | Action::ViewAlt => view::handle(state, action, context, terminal_backend),
        Action::Edit => edit::handle(state, context),
        Action::Copy => copy::handle(state, context),
        Action::Move => move_rename::handle(state, context),
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
        _ => false,
    }
}
