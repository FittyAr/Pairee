//! What each file action needs from the panel sources (local disk, SFTP,
//! archive), so unsupported actions show one clear message up front
//! instead of failing half-way.

use crate::app::state::{AppState, PopupType};
use crate::config::localization::t;
use crate::fs::vfs::Capability;
use crate::keybindings::Action;

/// Capability the active panel needs for `action`, and the one the
/// passive panel needs (copy/move destinations).
fn requirements(action: &Action) -> (Option<Capability>, Option<Capability>) {
    use Capability::*;
    match action {
        Action::WipeFile
        | Action::CreateLink
        | Action::ApplyCommand
        | Action::DescribeFile
        | Action::CompressFiles
        | Action::ExtractArchive
        | Action::ArchiveCommands => (Some(LocalTools), None),
        Action::Edit => (Some(Write), None),
        Action::FileAttributes => (Some(Attributes), None),
        Action::MkDir => (Some(MkDir), None),
        Action::Delete => (Some(Remove), None),
        Action::Rename | Action::MultiRename => (Some(Rename), None),
        Action::Copy => (None, Some(Write)),
        Action::Move => (Some(Remove), Some(Write)),
        _ => (None, None),
    }
}

/// `true` (after showing why) when the panels cannot run `action`.
pub fn refuse_unsupported(state: &mut AppState, action: &Action) -> bool {
    let (active, passive) = requirements(action);
    let active_ok = active.is_none_or(|c| state.get_active_panel().source.capabilities().allows(c));
    let passive_ok =
        passive.is_none_or(|c| state.get_passive_panel().source.capabilities().allows(c));
    // Archives are copied in and out; moving would delete from them.
    let moves_archive = matches!(action, Action::Move)
        && (state.get_active_panel().source.archive().is_some()
            || state.get_passive_panel().source.archive().is_some());
    // An archive inside an archive is only browsed and viewed.
    let copies_nested = matches!(action, Action::Copy)
        && state
            .get_active_panel()
            .source
            .archive()
            .is_some_and(|a| a.parent().is_some());
    if active_ok && passive_ok && !moves_archive && !copies_nested {
        return false;
    }
    state
        .dialogs
        .replace(PopupType::Info(t("vfs_action_unsupported")));
    true
}
