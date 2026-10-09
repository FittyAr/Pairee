//! Load and **validate** keymaps using the `keybinds` crate.
//!
//! Every context (panels, editor, viewer, lists) is built the same way from
//! its preset section. Layers, lowest priority first: the preset and every
//! preset it `extends` (root first), shipped defaults for actions an older
//! on-disk copy of a built-in preset lacks, setting layers,
//! `[overrides.all]`, then `[overrides.<preset>]`. See [`assign`] for how
//! layers combine.

pub mod assign;
pub mod disk;
pub mod layers;
pub mod report;
mod validate;

pub use crate::keybindings::chord::normalize_user_chord;
pub use layers::{FileLayer, Origin, Table};
pub use report::KeymapLoadReport;

use super::actions::Action;
use super::embedded::{self, normalize_preset_name};
use super::keymap::ContextKeymap;
use super::options::KeymapOptions;
use super::plugin_commands::{self, PluginCommand, PluginCommandId};
use super::registry::Bindable;
use super::screens::{EditorAction, ListAction, ViewerAction};
use crate::config::AppConfig;
use crate::config::keybindings::{KeybindingsConfig, OverrideTable};
use assign::Assignments;
use layers::{PresetSource, resolve_chain};
use std::collections::BTreeMap;

/// Preset used when the configured one does not exist.
const FALLBACK_PRESET: &str = "norton";

/// Everything that decides the active keymap.
pub struct KeymapSpec<'a> {
    pub preset: &'a str,
    pub keybindings: &'a KeybindingsConfig,
    /// The yazi workflow setting binds `s` / `v` to its sort and view menus.
    pub yazi_letters: bool,
    /// Commands of the loaded plugins, with the keys they suggest.
    pub plugins: Vec<(PluginCommandId, PluginCommand)>,
}

impl<'a> KeymapSpec<'a> {
    pub fn from_config(config: &'a AppConfig) -> Self {
        Self {
            preset: &config.keybindings.preset,
            keybindings: &config.keybindings,
            yazi_letters: config.settings.enable_yazi_workflow,
            plugins: plugin_commands::active(),
        }
    }
}

/// The validated keymaps of every context.
pub struct LoadedKeymap {
    pub panels: ContextKeymap<Action>,
    pub editor: ContextKeymap<EditorAction>,
    pub viewer: ContextKeymap<ViewerAction>,
    pub list: ContextKeymap<ListAction>,
    pub options: KeymapOptions,
    pub report: KeymapLoadReport,
}

/// Builds the keymap of `spec` from the presets shipped with Pairee only
/// (the plugin developer checks).
pub fn build_shipped_keymap(spec: &KeymapSpec) -> LoadedKeymap {
    build_keymap(spec, &embedded_source)
}

/// Loads the keymap `spec` describes from the preset files on disk.
pub fn load_keymap(spec: &KeymapSpec) -> LoadedKeymap {
    build_keymap(spec, &disk::find_preset_toml)
}

/// The layers of every context, in application order.
struct Plan {
    chain: Vec<FileLayer>,
    /// Shipped layers of a built-in preset, for actions an older on-disk
    /// copy does not name.
    defaults: Option<Vec<FileLayer>>,
    /// One suggestion layer per plugin, by plugin name.
    plugins: Vec<FileLayer>,
    extra: Vec<FileLayer>,
    options: KeymapOptions,
}

/// Builds the keymap of `spec`, reading preset files through `source`.
pub fn build_keymap(spec: &KeymapSpec, source: PresetSource) -> LoadedKeymap {
    let mut report = KeymapLoadReport::default();
    let name = normalize_preset_name(spec.preset);
    let chain = resolve_chain(&name, source, &mut report).unwrap_or_else(|| {
        report.warnings.push(format!(
            "Preset '{}' not found — falling back to {FALLBACK_PRESET}",
            spec.preset
        ));
        resolve_chain(FALLBACK_PRESET, &embedded_source, &mut report).unwrap_or_default()
    });
    let defaults = embedded::preset_toml(&name).and_then(|_| {
        resolve_chain(&name, &embedded_source, &mut KeymapLoadReport::default()).map(|c| c.layers)
    });
    let plan = Plan {
        chain: chain.layers,
        defaults,
        plugins: plugin_layers(&spec.plugins, &name),
        extra: extra_layers(spec, &name),
        options: chain.options,
    };
    let loaded = LoadedKeymap {
        panels: build_context(&plan, &mut report),
        editor: build_context(&plan, &mut report),
        viewer: build_context(&plan, &mut report),
        list: build_context(&plan, &mut report),
        options: plan.options,
        report: KeymapLoadReport::default(),
    };
    report.bound_count = loaded.panels.rows().len()
        + loaded.editor.rows().len()
        + loaded.viewer.rows().len()
        + loaded.list.rows().len();
    if loaded.panels.rows().is_empty() {
        report
            .errors
            .push("No key bindings loaded — keymap is empty after validation".into());
    }
    LoadedKeymap { report, ..loaded }
}

/// Applies every layer of `plan` to the section of `B` and validates it.
fn build_context<B: Bindable>(plan: &Plan, report: &mut KeymapLoadReport) -> ContextKeymap<B> {
    let leader = plan.options.leader.as_deref();
    let mut assignments = Assignments::<B>::default();
    for layer in &plan.chain {
        assignments.apply(&layer.section(B::SECTION), leader, report);
    }
    if let Some(layers) = &plan.defaults {
        let mut silent = KeymapLoadReport::default();
        let mut defaults = Assignments::<B>::default();
        for layer in layers {
            defaults.apply(&layer.section(B::SECTION), leader, &mut silent);
        }
        assignments.fill_missing_from(&defaults);
    }
    for layer in &plan.plugins {
        assignments.suggest(&layer.section(B::SECTION), leader, report);
    }
    for layer in &plan.extra {
        assignments.apply(&layer.section(B::SECTION), leader, report);
    }
    validate::check_prefixes(assignments.rows(), report);
    validate::check_robustness(assignments.rows(), report);
    let (keybinds, rows) = assignments.finish();
    ContextKeymap::new(keybinds, rows, plan.options.sequence_timeout())
}

fn embedded_source(name: &str, _: &mut KeymapLoadReport) -> Option<String> {
    embedded::preset_toml(name).map(str::to_string)
}

/// The keys each plugin suggests for `preset`, one layer per plugin in name
/// order, so between plugins the first by name keeps a contested chord.
fn plugin_layers(commands: &[(PluginCommandId, PluginCommand)], preset: &str) -> Vec<FileLayer> {
    let mut by_plugin: BTreeMap<&str, Table> = BTreeMap::new();
    for (_, command) in commands {
        if let Some(keys) = command.keys_for(preset) {
            by_plugin
                .entry(&command.plugin)
                .or_default()
                .insert(command.keymap_id(), keys.to_string());
        }
    }
    by_plugin
        .into_iter()
        .map(|(plugin, table)| FileLayer::panels(Origin::Plugin(plugin.to_string()), table))
        .collect()
}

/// Setting layers, then the user's overrides for `preset`.
fn extra_layers(spec: &KeymapSpec, preset: &str) -> Vec<FileLayer> {
    let mut out = Vec::new();
    if spec.yazi_letters {
        out.push(FileLayer::panels(
            Origin::Setting("int_yazi_workflow"),
            BTreeMap::from([
                ("sort_menu".to_string(), "s".to_string()),
                ("view_mode_menu".to_string(), "v".to_string()),
            ]),
        ));
    }
    for (key, table) in spec.keybindings.overrides_for(preset) {
        out.push(FileLayer {
            origin: Origin::Override(key.to_string()),
            sections: split_sections(table),
        });
    }
    out
}

/// Override ids name their section with a prefix (`editor.save`); plain
/// ids are panel actions.
pub fn split_sections(table: &OverrideTable) -> BTreeMap<String, Table> {
    let mut sections: BTreeMap<String, Table> = BTreeMap::new();
    for (id, keys) in table {
        let (section, id) = match id.split_once('.') {
            Some((section, rest)) if ["panels", "editor", "viewer", "list"].contains(&section) => {
                (section, rest)
            }
            _ => ("panels", id.as_str()),
        };
        sections
            .entry(section.to_string())
            .or_default()
            .insert(id.to_string(), keys.clone());
    }
    sections
}

#[cfg(test)]
mod fidelity_tests;
#[cfg(test)]
mod tests;
