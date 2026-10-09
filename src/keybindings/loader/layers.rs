//! Keymap layers and the `extends` chain of preset files.

use super::report::KeymapLoadReport;
use crate::keybindings::options::{KeymapOptions, OptionsTable};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::fmt;

/// Longest `extends` chain accepted before assuming a mistake.
const MAX_CHAIN: usize = 8;

/// Where a binding came from, for reports and the shortcuts modal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Origin {
    /// A preset file (`base`, `norton`, a user preset…).
    Preset(String),
    /// A setting that adds bindings (e.g. the yazi letter keys).
    Setting(&'static str),
    /// `[overrides.<key>]` of `keybindings.toml` (`all` or a preset).
    Override(String),
    /// Keys a plugin suggests in its manifest.
    Plugin(String),
}

impl fmt::Display for Origin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Preset(name) => write!(f, "preset '{name}'"),
            Self::Setting(key) => write!(f, "setting '{key}'"),
            Self::Override(key) => write!(f, "overrides.{key}"),
            Self::Plugin(name) => write!(f, "plugin '{name}'"),
        }
    }
}

/// `action id → chords` of one section.
pub type Table = BTreeMap<String, String>;

/// One section's bindings, applied over the layers before it.
#[derive(Debug, Clone)]
pub struct Layer {
    pub origin: Origin,
    pub bindings: Table,
}

/// A layer of every section (a preset file, an override table...).
#[derive(Debug, Clone)]
pub struct FileLayer {
    pub origin: Origin,
    /// Section name (`panels`, `editor`, `viewer`, `list`) → table.
    pub sections: BTreeMap<String, Table>,
}

impl FileLayer {
    /// A layer of the panels section only.
    pub fn panels(origin: Origin, bindings: Table) -> Self {
        Self {
            origin,
            sections: BTreeMap::from([("panels".to_string(), bindings)]),
        }
    }

    /// The bindings of `section` as a layer.
    pub fn section(&self, section: &str) -> Layer {
        Layer {
            origin: self.origin.clone(),
            bindings: self.sections.get(section).cloned().unwrap_or_default(),
        }
    }
}

/// A preset file (v2). `[bindings]` is the pre-v2 name of `[panels]`.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct PresetFile {
    extends: Option<String>,
    #[serde(default)]
    options: OptionsTable,
    #[serde(default, alias = "bindings")]
    panels: Table,
    #[serde(default)]
    editor: Table,
    #[serde(default)]
    viewer: Table,
    #[serde(default)]
    list: Table,
}

impl PresetFile {
    fn into_sections(self) -> BTreeMap<String, Table> {
        BTreeMap::from([
            ("panels".to_string(), self.panels),
            ("editor".to_string(), self.editor),
            ("viewer".to_string(), self.viewer),
            ("list".to_string(), self.list),
        ])
    }
}

/// Loads the TOML of a preset by name (`None` when it does not exist).
pub type PresetSource<'a> = &'a dyn Fn(&str, &mut KeymapLoadReport) -> Option<String>;

/// A preset with its ancestors: layers root first, options merged.
#[derive(Debug, Default)]
pub struct PresetChain {
    pub layers: Vec<FileLayer>,
    pub options: KeymapOptions,
}

/// Resolves `name` and every preset it `extends`. `None` when `name`
/// itself does not exist; a broken ancestor is reported and skipped.
pub fn resolve_chain(
    name: &str,
    source: PresetSource,
    report: &mut KeymapLoadReport,
) -> Option<PresetChain> {
    let mut files: Vec<(String, PresetFile)> = Vec::new();
    let mut next = Some(name.to_string());
    while let Some(current) = next.take() {
        if files.iter().any(|(n, _)| *n == current) || files.len() == MAX_CHAIN {
            report.errors.push(format!(
                "Keymap 'extends' chain loops or is too deep at '{current}'"
            ));
            break;
        }
        let Some(src) = source(&current, report) else {
            if files.is_empty() {
                return None;
            }
            report.errors.push(format!(
                "Preset '{current}' (extended by another preset) not found"
            ));
            break;
        };
        let file = match toml::from_str::<PresetFile>(&src) {
            Ok(file) => file,
            Err(e) => {
                report.errors.push(format!(
                    "Failed to parse keymap for preset '{current}': {e}"
                ));
                PresetFile::default()
            }
        };
        next = file.extends.clone();
        files.push((current, file));
    }
    let mut chain = PresetChain::default();
    for (name, file) in files.into_iter().rev() {
        chain.options.overlay(&file.options);
        chain.layers.push(FileLayer {
            origin: Origin::Preset(name),
            sections: file.into_sections(),
        });
    }
    Some(chain)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source(
        files: &'static [(&'static str, &'static str)],
    ) -> impl Fn(&str, &mut KeymapLoadReport) -> Option<String> {
        move |name, _| {
            files
                .iter()
                .find(|(n, _)| *n == name)
                .map(|(_, src)| src.to_string())
        }
    }

    #[test]
    fn chain_is_root_first_and_merges_options() {
        let src = source(&[
            (
                "root",
                "[options]\nleader = \"Space\"\n[panels]\ncopy = \"F5\"",
            ),
            ("mid", "extends = \"root\"\n[bindings]\nmove = \"F6\""),
            ("top", "extends = \"mid\"\n[options]\ntyping = \"commands\""),
        ]);
        let mut report = KeymapLoadReport::default();
        let chain = resolve_chain("top", &src, &mut report).unwrap();
        let names: Vec<String> = chain.layers.iter().map(|l| l.origin.to_string()).collect();
        assert_eq!(names, ["preset 'root'", "preset 'mid'", "preset 'top'"]);
        assert_eq!(chain.layers[1].section("panels").bindings["move"], "F6");
        assert_eq!(chain.options.leader.as_deref(), Some("Space"));
        assert!(report.errors.is_empty(), "{:?}", report.errors);
    }

    #[test]
    fn loops_and_missing_parents_are_errors() {
        let src = source(&[
            ("a", "extends = \"b\""),
            ("b", "extends = \"a\""),
            ("c", "extends = \"nope\""),
        ]);
        let mut report = KeymapLoadReport::default();
        assert_eq!(
            resolve_chain("a", &src, &mut report).unwrap().layers.len(),
            2
        );
        assert!(report.errors[0].contains("loops"));
        assert!(resolve_chain("c", &src, &mut report).is_some());
        assert!(report.errors[1].contains("'nope'"));
        assert!(resolve_chain("zzz", &src, &mut report).is_none());
    }

    #[test]
    fn unknown_sections_are_rejected() {
        let src = source(&[("x", "[not_a_section]\ncopy = \"F5\"")]);
        let mut report = KeymapLoadReport::default();
        resolve_chain("x", &src, &mut report);
        assert!(
            report.errors[0].contains("Failed to parse"),
            "{:?}",
            report.errors
        );
    }
}
