//! Rows of the keyboard-shortcuts modal: every command of every context,
//! with the keys it has in a keymap (none for unbound commands) and marks
//! for the user's changes, plugin conflicts and fragile keys.

use crate::config::keybindings::KeybindingsConfig;
use crate::keybindings::Action;
use crate::keybindings::catalog::Category;
use crate::keybindings::chord::fragility;
use crate::keybindings::keymap::ContextKeymap;
use crate::keybindings::loader::{KeymapLoadReport, LoadedKeymap, Origin};
use crate::keybindings::plugin_commands;
use crate::keybindings::registry::Bindable;
use crate::keybindings::screens::{EditorAction, ListAction, ViewerAction};
use std::collections::HashSet;

/// The contexts the modal shows as tabs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextTab {
    Panels,
    Editor,
    Viewer,
    List,
}

impl ContextTab {
    pub const ALL: [Self; 4] = [Self::Panels, Self::Editor, Self::Viewer, Self::List];

    /// Preset section of the context.
    pub fn section(self) -> &'static str {
        match self {
            Self::Panels => "panels",
            Self::Editor => "editor",
            Self::Viewer => "viewer",
            Self::List => "list",
        }
    }

    pub fn label_key(self) -> &'static str {
        match self {
            Self::Panels => "shortcuts_tab_panels",
            Self::Editor => "shortcuts_tab_editor",
            Self::Viewer => "shortcuts_tab_viewer",
            Self::List => "shortcuts_tab_list",
        }
    }

    /// The tab after (`step` = 1) or before (`step` = -1) this one, wrapping.
    pub fn step(self, step: isize) -> Self {
        let i = Self::ALL.iter().position(|t| *t == self).unwrap_or(0) as isize;
        Self::ALL[(i + step).rem_euclid(Self::ALL.len() as isize) as usize]
    }

    /// Id of command `id` in `keybindings.toml` overrides.
    pub fn override_id(self, id: &str) -> String {
        match self {
            Self::Panels => id.to_string(),
            other => format!("{}.{id}", other.section()),
        }
    }
}

/// One command of one context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShortcutRow {
    pub tab: ContextTab,
    /// Id in the context (`copy`, `save`, `plugin.blame.toggle`).
    pub id: String,
    pub label: String,
    pub category: Category,
    /// Bound chords, terminal-robust ones first.
    pub chords: Vec<String>,
    /// A user override decides this command's keys.
    pub user: bool,
    /// Plugin that declares the command.
    pub plugin: Option<String>,
    /// A key the plugin suggested was taken.
    pub conflict: bool,
    /// Every bound chord needs a capable terminal.
    pub fragile: bool,
    /// The panel action to run from the modal.
    pub run: Option<Action>,
}

impl ShortcutRow {
    pub fn override_id(&self) -> String {
        self.tab.override_id(&self.id)
    }

    /// Marker shown before the row.
    pub fn mark(&self) -> &'static str {
        if self.conflict {
            "⚠"
        } else if self.user {
            "●"
        } else if self.chords.is_empty() {
            "∅"
        } else if self.fragile {
            "≈"
        } else {
            " "
        }
    }
}

/// Every row of `loaded` (the keymap of `preset`), by context, then
/// category, then label. `keybindings` tells which rows the user changed.
pub fn build_rows(
    loaded: &LoadedKeymap,
    keybindings: &KeybindingsConfig,
    preset: &str,
) -> Vec<ShortcutRow> {
    let user: HashSet<String> = keybindings
        .overrides_for(preset)
        .into_iter()
        .flat_map(|(_, table)| table.keys().cloned())
        .collect();
    let ctx = RowCtx {
        report: &loaded.report,
        user: &user,
    };
    let mut panel_commands = Action::all();
    panel_commands.extend(
        plugin_commands::active()
            .into_iter()
            .map(|(id, _)| Action::Plugin(id)),
    );
    let mut rows = context_rows(
        ContextTab::Panels,
        &loaded.panels,
        panel_commands,
        Some,
        &ctx,
    );
    rows.extend(context_rows(
        ContextTab::Editor,
        &loaded.editor,
        EditorAction::all(),
        |_| None,
        &ctx,
    ));
    rows.extend(context_rows(
        ContextTab::Viewer,
        &loaded.viewer,
        ViewerAction::all(),
        |_| None,
        &ctx,
    ));
    rows.extend(context_rows(
        ContextTab::List,
        &loaded.list,
        ListAction::all(),
        |_| None,
        &ctx,
    ));
    rows
}

/// What every row needs besides its keymap.
struct RowCtx<'a> {
    report: &'a KeymapLoadReport,
    /// Override ids the user set for this preset.
    user: &'a HashSet<String>,
}

fn context_rows<B: Bindable>(
    tab: ContextTab,
    keymap: &ContextKeymap<B>,
    commands: Vec<B>,
    run: impl Fn(B) -> Option<Action>,
    ctx: &RowCtx,
) -> Vec<ShortcutRow> {
    let mut rows: Vec<ShortcutRow> = commands
        .into_iter()
        .map(|command| {
            let bound: Vec<_> = keymap
                .rows()
                .iter()
                .filter(|r| r.command == command)
                .collect();
            let id = command.id().to_string();
            let conflict_tag = format!(" for {id} (");
            ShortcutRow {
                tab,
                label: command.label(),
                category: command.category(),
                chords: keymap.keys_for(command).to_vec(),
                user: ctx.user.contains(&tab.override_id(&id)),
                plugin: bound.iter().find_map(|r| match &r.origin {
                    Origin::Plugin(name) => Some(name.clone()),
                    _ => None,
                }),
                conflict: ctx
                    .report
                    .conflicts
                    .iter()
                    .any(|c| c.contains(&conflict_tag)),
                fragile: !bound.is_empty() && bound.iter().all(|r| fragility(&r.seq).is_some()),
                run: run(command),
                id,
            }
        })
        .collect();
    rows.sort_by(|a, b| {
        a.category
            .cmp(&b.category)
            .then_with(|| a.label.cmp(&b.label))
    });
    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::keybindings::ALL_PRESETS;
    use crate::keybindings::loader::{KeymapSpec, build_shipped_keymap};

    fn rows(overrides: &[(&str, &str)]) -> Vec<ShortcutRow> {
        let mut keybindings = KeybindingsConfig::default();
        for (id, keys) in overrides {
            keybindings.set_override(ALL_PRESETS, id, keys);
        }
        let loaded = build_shipped_keymap(&KeymapSpec {
            preset: "norton",
            keybindings: &keybindings,
            yazi_letters: false,
            plugins: Vec::new(),
        });
        build_rows(&loaded, &keybindings, "norton")
    }

    fn find<'a>(rows: &'a [ShortcutRow], tab: ContextTab, id: &str) -> &'a ShortcutRow {
        rows.iter().find(|r| r.tab == tab && r.id == id).unwrap()
    }

    #[test]
    fn every_context_lists_bound_and_unbound_commands() {
        let rows = rows(&[("viewer.encoding", "")]);
        let copy = find(&rows, ContextTab::Panels, "copy");
        assert_eq!(copy.chords, ["F5"]);
        assert_eq!(copy.run, Some(Action::Copy));
        assert_eq!(copy.mark(), " ");
        let save = find(&rows, ContextTab::Editor, "save");
        assert_eq!(save.override_id(), "editor.save");
        let encoding = find(&rows, ContextTab::Viewer, "encoding");
        assert!(encoding.chords.is_empty() && encoding.user);
        assert_eq!(encoding.mark(), "●");
        assert!(
            rows.iter()
                .any(|r| r.tab == ContextTab::Panels && r.chords.is_empty())
        );
    }

    #[test]
    fn fragile_rows_are_marked() {
        let rows = rows(&[]);
        assert_eq!(
            find(&rows, ContextTab::Panels, "insert_name_to_cli").mark(),
            "≈"
        );
    }
}
