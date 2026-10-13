//! Which actions a plugin may fire with `pairee.emit`.
//!
//! * **Trusted** plugins may fire any action. Destructive ones run exactly
//!   like a key press, so they go through the usual confirmation dialogs
//!   (and honour `confirm_delete` and the other confirmation settings).
//! * **Untrusted** plugins may only fire the actions listed in
//!   [`untrusted_may_emit`]: navigation, view, sort, selection and help.
//!   Nothing that copies, moves, deletes, writes, runs commands, changes
//!   settings or connects anywhere is on the list, and an untrusted plugin
//!   may only fire its own commands (`plugin.<its name>.*`), never those of
//!   another (possibly trusted) plugin.

use super::request::PluginOrigin;
use crate::keybindings::Action;
use crate::keybindings::registry::Bindable;

/// `true` when `origin` may fire `action`.
pub fn may_emit(origin: &PluginOrigin, action: Action) -> bool {
    origin.trusted || untrusted_may_emit(&origin.name, action)
}

/// The explicit allowlist for untrusted plugins.
pub fn untrusted_may_emit(plugin: &str, action: Action) -> bool {
    use Action as A;
    match action {
        A::Plugin(_) => action
            .id()
            .strip_prefix("plugin.")
            .and_then(|rest| rest.strip_prefix(plugin))
            .is_some_and(|rest| rest.starts_with('.')),
        A::MoveUp
        | A::MoveDown
        | A::PageUp
        | A::PageDown
        | A::HalfPageUp
        | A::HalfPageDown
        | A::GoToTop
        | A::GoToBottom
        | A::ChangePanel
        | A::FocusLeftPanel
        | A::FocusRightPanel
        | A::GoParent
        | A::GoHome
        | A::GoRoot
        | A::HistoryBack
        | A::HistoryForward
        | A::Refresh
        | A::RereadPanel
        | A::NextTab
        | A::PrevTab
        | A::GoToTab(_)
        | A::PanelViewBrief
        | A::PanelViewMedium
        | A::PanelViewFull
        | A::PanelViewWide
        | A::PanelViewDetailed
        | A::PanelViewDescriptions
        | A::PanelViewFileOwners
        | A::PanelViewFileLinks
        | A::PanelViewAltFull
        | A::CyclePanelView
        | A::InfoPanel
        | A::QuickView
        | A::TreeView
        | A::ToggleHidden
        | A::ToggleLongNames
        | A::SortByName
        | A::SortByExtension
        | A::SortByWriteTime
        | A::SortBySize
        | A::SortUnsorted
        | A::SortByCreationTime
        | A::SortByAccessTime
        | A::SortByDescription
        | A::SortByOwner
        | A::ToggleSortReverse
        | A::CycleSort
        | A::SelectItem
        | A::SelectAll
        | A::UnselectAll
        | A::InvertSelection
        | A::FindInPanel
        | A::FindNext
        | A::FindPrev
        | A::View
        | A::Help
        | A::About
        | A::WhichKey
        | A::CommandPalette => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn untrusted() -> PluginOrigin {
        PluginOrigin {
            name: "demo".into(),
            trusted: false,
        }
    }

    #[test]
    fn untrusted_cannot_fire_destructive_actions() {
        for action in [
            Action::Delete,
            Action::DeletePermanent,
            Action::Trash,
            Action::WipeFile,
            Action::Move,
            Action::Copy,
            Action::Rename,
            Action::MultiRename,
            Action::Paste,
            Action::PasteOverwrite,
            Action::ApplyCommand,
            Action::UserMenu,
            Action::SaveSetup,
            Action::SshConnect,
            Action::ExtractArchive,
            Action::InstallDevPlugin,
            Action::Quit,
        ] {
            assert!(!may_emit(&untrusted(), action), "{action:?}");
        }
    }

    #[test]
    fn untrusted_may_navigate_and_trusted_may_do_anything() {
        assert!(may_emit(&untrusted(), Action::MoveDown));
        assert!(may_emit(&untrusted(), Action::GoToTab(2)));
        let trusted = PluginOrigin {
            trusted: true,
            ..untrusted()
        };
        assert!(may_emit(&trusted, Action::DeletePermanent));
    }

    #[test]
    fn untrusted_may_only_fire_its_own_commands() {
        use crate::keybindings::plugin_commands::{self, PluginCommand};
        let cmd = |plugin: &str| PluginCommand {
            plugin: plugin.into(),
            command: "go".into(),
            title: "Go".into(),
            keys: Default::default(),
        };
        plugin_commands::register("emitpol-own", vec![cmd("emitpol-own")]);
        plugin_commands::register("emitpol-other", vec![cmd("emitpol-other")]);
        let own = Action::from_id("plugin.emitpol-own.go").unwrap();
        let other = Action::from_id("plugin.emitpol-other.go").unwrap();
        let origin = PluginOrigin {
            name: "emitpol-own".into(),
            trusted: false,
        };
        assert!(may_emit(&origin, own));
        assert!(!may_emit(&origin, other));
        let prefix = PluginOrigin {
            name: "emitpol".into(),
            trusted: false,
        };
        assert!(!may_emit(&prefix, own));
        plugin_commands::unregister("emitpol-own");
        plugin_commands::unregister("emitpol-other");
    }
}
