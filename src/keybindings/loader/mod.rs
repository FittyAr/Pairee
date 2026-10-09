//! Load and **validate** keymaps using the `keybinds` crate.
//!
//! Layers, lowest priority first: the preset and every preset it `extends`
//! (root first), shipped defaults for actions an older on-disk copy of a
//! built-in preset lacks, setting layers, `[overrides.all]`, then
//! `[overrides.<preset>]`. See [`assign`] for how layers combine.

pub mod assign;
pub mod disk;
pub mod layers;
pub mod report;
mod validate;

pub use crate::keybindings::chord::normalize_user_chord;
pub use layers::{Layer, Origin};
pub use report::KeymapLoadReport;

use super::actions::Action;
use super::embedded::{self, normalize_preset_name};
use super::options::KeymapOptions;
use crate::config::AppConfig;
use crate::config::keybindings::KeybindingsConfig;
use assign::{Assignments, Row};
use keybinds::Keybinds;
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
}

impl<'a> KeymapSpec<'a> {
    pub fn from_config(config: &'a AppConfig) -> Self {
        Self {
            preset: &config.keybindings.preset,
            keybindings: &config.keybindings,
            yazi_letters: config.settings.enable_yazi_workflow,
        }
    }
}

/// A validated keymap ready for the resolver.
pub struct LoadedKeymap {
    pub keybinds: Keybinds<Action>,
    pub rows: Vec<Row<Action>>,
    pub options: KeymapOptions,
    pub report: KeymapLoadReport,
}

/// Loads the keymap `spec` describes from the preset files on disk.
pub fn load_keymap(spec: &KeymapSpec) -> LoadedKeymap {
    build_keymap(spec, &disk::find_preset_toml)
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
    let leader = chain.options.leader.as_deref();
    let mut assignments = Assignments::default();
    for layer in &chain.layers {
        assignments.apply(layer, leader, &mut report);
    }
    if let Some(defaults) = shipped_defaults(&name, leader) {
        assignments.fill_missing_from(&defaults);
    }
    for layer in extra_layers(spec, &name) {
        assignments.apply(&layer, leader, &mut report);
    }
    validate::check_prefixes(assignments.rows(), &mut report);
    validate::check_robustness(assignments.rows(), &mut report);
    let (keybinds, rows) = assignments.finish();
    report.bound_count = rows.len();
    if rows.is_empty() {
        report
            .errors
            .push("No key bindings loaded — keymap is empty after validation".into());
    }
    LoadedKeymap {
        keybinds,
        rows,
        options: chain.options,
        report,
    }
}

fn embedded_source(name: &str, _: &mut KeymapLoadReport) -> Option<String> {
    embedded::preset_toml(name).map(str::to_string)
}

/// The shipped bindings of built-in preset `name` (problems in shipped
/// files are caught by tests, so they are not reported here).
fn shipped_defaults(name: &str, leader: Option<&str>) -> Option<Assignments<Action>> {
    embedded::preset_toml(name)?;
    let mut silent = KeymapLoadReport::default();
    let chain = resolve_chain(name, &embedded_source, &mut silent)?;
    let mut defaults = Assignments::default();
    for layer in &chain.layers {
        defaults.apply(layer, leader, &mut silent);
    }
    Some(defaults)
}

/// Setting layers, then the user's overrides for `preset`.
fn extra_layers(spec: &KeymapSpec, preset: &str) -> Vec<Layer> {
    let mut out = Vec::new();
    if spec.yazi_letters {
        out.push(Layer {
            origin: Origin::Setting("int_yazi_workflow"),
            bindings: BTreeMap::from([
                ("sort_menu".to_string(), "s".to_string()),
                ("view_mode_menu".to_string(), "v".to_string()),
            ]),
        });
    }
    for (key, table) in spec.keybindings.overrides_for(preset) {
        out.push(Layer {
            origin: Origin::Override(key.to_string()),
            bindings: table.clone(),
        });
    }
    out
}

#[cfg(test)]
mod tests;
