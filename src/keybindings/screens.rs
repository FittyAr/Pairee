//! Commands of the editor and viewer screens and of list popups, bound in
//! the `[editor]`, `[viewer]` and `[list]` sections of a preset.
//!
//! Typing and cursor motions with selection stay with the editor itself:
//! only commands are bindable.

use super::catalog::Category;
use super::registry::Bindable;
use std::fmt::Debug;
use std::hash::Hash;

/// Built-in editor commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EditorAction {
    Save,
    SaveAs,
    Reload,
    Search,
    SearchNext,
    OpenViewer,
    Discard,
    /// Clears the selection, else closes the editor (asking when modified).
    Quit,
    ToggleBlockMode,
    Undo,
    Redo,
    Copy,
    Cut,
    Paste,
    SelectAll,
}

/// Built-in viewer commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ViewerAction {
    LineUp,
    LineDown,
    PageUp,
    PageDown,
    Top,
    Bottom,
    ToggleHex,
    Edit,
    Search,
    SearchNext,
    Encoding,
    /// Stops a running search, else closes the viewer.
    Quit,
}

/// Cursor keys of list popups (menus, history lists, pickers...). A key
/// bound here acts as the matching arrow / Enter / Esc key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ListAction {
    Up,
    Down,
    PageUp,
    PageDown,
    First,
    Last,
    Activate,
    Close,
}

/// One catalogued screen command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScreenDef<B> {
    pub id: &'static str,
    pub command: B,
    pub category: Category,
    pub essential: bool,
    /// Short F-key bar label (i18n key), for commands that sit on F-keys.
    pub fkey: Option<&'static str>,
}

const fn cmd<B>(id: &'static str, command: B, category: Category) -> ScreenDef<B> {
    ScreenDef {
        id,
        command,
        category,
        essential: false,
        fkey: None,
    }
}

const fn on_fkey<B: Copy>(def: ScreenDef<B>, label: &'static str) -> ScreenDef<B> {
    ScreenDef {
        fkey: Some(label),
        ..def
    }
}

const fn must<B: Copy>(def: ScreenDef<B>) -> ScreenDef<B> {
    ScreenDef {
        essential: true,
        ..def
    }
}

use Category as C;
use EditorAction as E;
use ListAction as L;
use ViewerAction as V;

pub const EDITOR_CATALOG: &[ScreenDef<EditorAction>] = &[
    must(on_fkey(cmd("save", E::Save, C::Files), "fkey_ed_save")),
    on_fkey(cmd("save_as", E::SaveAs, C::Files), "fkey_ed_save_as"),
    cmd("reload", E::Reload, C::Files),
    on_fkey(cmd("search", E::Search, C::Search), "fkey_ed_search"),
    on_fkey(cmd("search_next", E::SearchNext, C::Search), "fkey_ed_next"),
    on_fkey(cmd("open_viewer", E::OpenViewer, C::View), "fkey_ed_hex"),
    on_fkey(cmd("discard", E::Discard, C::Files), "fkey_ed_discard"),
    must(on_fkey(cmd("quit", E::Quit, C::System), "fkey_ed_quit")),
    cmd("toggle_block_mode", E::ToggleBlockMode, C::Selection),
    cmd("undo", E::Undo, C::Files),
    cmd("redo", E::Redo, C::Files),
    cmd("copy", E::Copy, C::Clipboard),
    cmd("cut", E::Cut, C::Clipboard),
    cmd("paste", E::Paste, C::Clipboard),
    cmd("select_all", E::SelectAll, C::Selection),
];

pub const VIEWER_CATALOG: &[ScreenDef<ViewerAction>] = &[
    cmd("line_up", V::LineUp, C::Navigation),
    cmd("line_down", V::LineDown, C::Navigation),
    cmd("page_up", V::PageUp, C::Navigation),
    cmd("page_down", V::PageDown, C::Navigation),
    cmd("top", V::Top, C::Navigation),
    cmd("bottom", V::Bottom, C::Navigation),
    on_fkey(cmd("toggle_hex", V::ToggleHex, C::View), "fkey_vw_hex"),
    on_fkey(cmd("edit", V::Edit, C::Files), "fkey_edit"),
    on_fkey(cmd("search", V::Search, C::Search), "fkey_vw_search"),
    cmd("search_next", V::SearchNext, C::Search),
    cmd("encoding", V::Encoding, C::View),
    must(on_fkey(cmd("quit", V::Quit, C::System), "fkey_vw_quit")),
];

pub const LIST_CATALOG: &[ScreenDef<ListAction>] = &[
    must(cmd("up", L::Up, C::Navigation)),
    must(cmd("down", L::Down, C::Navigation)),
    cmd("page_up", L::PageUp, C::Navigation),
    cmd("page_down", L::PageDown, C::Navigation),
    cmd("first", L::First, C::Navigation),
    cmd("last", L::Last, C::Navigation),
    must(cmd("activate", L::Activate, C::Navigation)),
    must(cmd("close", L::Close, C::System)),
];

/// A command enum with a catalogue; implements [`Bindable`].
pub trait ScreenCommand: Copy + Eq + Hash + Debug + 'static {
    /// Preset section and override prefix (`editor`, `viewer`, `list`).
    const SECTION: &'static str;
    fn catalog() -> &'static [ScreenDef<Self>];

    fn def(self) -> &'static ScreenDef<Self> {
        Self::catalog()
            .iter()
            .find(|d| d.command == self)
            .expect("every screen command is catalogued")
    }
}

impl ScreenCommand for EditorAction {
    const SECTION: &'static str = "editor";
    fn catalog() -> &'static [ScreenDef<Self>] {
        EDITOR_CATALOG
    }
}

impl ScreenCommand for ViewerAction {
    const SECTION: &'static str = "viewer";
    fn catalog() -> &'static [ScreenDef<Self>] {
        VIEWER_CATALOG
    }
}

impl ScreenCommand for ListAction {
    const SECTION: &'static str = "list";
    fn catalog() -> &'static [ScreenDef<Self>] {
        LIST_CATALOG
    }
}

impl<T: ScreenCommand> Bindable for T {
    const SECTION: &'static str = <T as ScreenCommand>::SECTION;

    fn from_id(id: &str) -> Option<Self> {
        T::catalog().iter().find(|d| d.id == id).map(|d| d.command)
    }

    fn id(self) -> &'static str {
        self.def().id
    }

    fn label_in(self, tr: &dyn Fn(&str) -> String) -> String {
        tr(&format!("action_{}_{}", T::SECTION, self.id()))
    }

    fn category(self) -> Category {
        self.def().category
    }

    fn essential(self) -> bool {
        self.def().essential
    }

    fn all() -> Vec<Self> {
        T::catalog().iter().map(|d| d.command).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::localization::translator::get_default_english_translation;

    fn check<T: ScreenCommand>() {
        let mut ids: Vec<&str> = T::catalog().iter().map(|d| d.id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(
            ids.len(),
            T::catalog().len(),
            "{} ids are unique",
            T::SECTION
        );
        for d in T::catalog() {
            assert_eq!(T::from_id(d.id), Some(d.command));
            let key = format!("action_{}_{}", T::SECTION, d.id);
            assert_ne!(get_default_english_translation(&key), key, "missing {key}");
            if let Some(fkey) = d.fkey {
                assert_ne!(
                    get_default_english_translation(fkey),
                    fkey,
                    "missing {fkey}"
                );
            }
        }
    }

    #[test]
    fn catalogues_are_unique_and_labelled() {
        check::<EditorAction>();
        check::<ViewerAction>();
        check::<ListAction>();
    }
}
