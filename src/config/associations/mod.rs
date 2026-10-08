pub mod defaults;
pub mod rule;

pub use defaults::get_default_rules;
pub use rule::AssocRule;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use super::write_atomic;

/// Holds all file association rules. Loaded from / saved to `associations.toml`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AssociationsConfig {
    pub rules: Vec<AssocRule>,
}

impl AssociationsConfig {
    /// Loads associations from disk; returns an empty config if the file is missing.
    pub fn load() -> Self {
        match Self::try_load() {
            Ok(mut config) => {
                if config.migrate()
                    && let Err(e) = config.save()
                {
                    log::warn!("Failed to refresh associations.toml: {}", e);
                }
                config
            }
            Err(_) => {
                let default_rules = Self::default_rules();
                if let Err(e) = default_rules.save() {
                    log::warn!("Failed to write default associations.toml: {}", e);
                }
                default_rules
            }
        }
    }

    fn try_load() -> Result<Self> {
        let path = associations_path();
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("Reading associations file {:?}", path))?;
        toml::from_str(&content).context("Deserializing associations.toml")
    }

    /// Persists the configuration to `<config_dir>/pairee/associations.toml`.
    pub fn save(&self) -> Result<()> {
        let path = associations_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).context("Creating config directory")?;
        }
        let toml_str = toml::to_string_pretty(self).context("Serializing associations")?;
        write_atomic(&path, toml_str.as_bytes())
            .with_context(|| format!("Writing associations file {:?}", path))
    }

    /// Upgrades rules written by older releases. Returns `true` when the rules
    /// changed and should be saved.
    ///
    /// - The original four-rule set is replaced by the current defaults.
    /// - Untouched default rules that launched an external text editor are
    ///   dropped: editing is done by the built-in editor only.
    fn migrate(&mut self) -> bool {
        let is_original_set = self.rules.len() == 4
            && self.rules[0].mask == "*.rs"
            && self.rules[1].mask == "*.toml"
            && self.rules[2].mask == "*.md"
            && self.rules[3].mask == "*.{zip,tar,gz,bz2,xz,7z}";
        if is_original_set {
            *self = Self::default_rules();
            return true;
        }
        let before = self.rules.len();
        self.rules.retain(|r| !defaults::is_legacy_editor_rule(r));
        self.rules.len() != before
    }

    /// Finds the first rule whose mask matches the given filename.
    pub fn find_rule(&self, filename: &str) -> Option<&AssocRule> {
        self.rules.iter().find(|r| r.matches(filename))
    }

    /// Returns a default set of common rules for a fresh install.
    pub fn default_rules() -> Self {
        Self {
            rules: get_default_rules(),
        }
    }
}

fn associations_path() -> PathBuf {
    crate::config::paths::get_config_dir().join("associations.toml")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_assoc_rule_matches() {
        let rule = AssocRule {
            mask: "*.rs".to_string(),
            open_cmd: "nano %f".to_string(),
            view_cmd: None,
        };
        assert!(rule.matches("main.rs"));
        assert!(!rule.matches("main.toml"));
    }

    #[test]
    fn test_resolve_open_cmd() {
        let rule = AssocRule {
            mask: "*.md".to_string(),
            open_cmd: "nano %f".to_string(),
            view_cmd: None,
        };
        let path = PathBuf::from("/home/user/README.md");
        let (prog, args) = rule.resolve_open_cmd(&path);
        assert_eq!(prog, "nano");
        assert_eq!(args, vec!["/home/user/README.md".to_string()]);
    }

    #[test]
    fn test_resolve_open_cmd_with_extra_args() {
        let rule = AssocRule {
            mask: "*.rs".to_string(),
            open_cmd: "code --new-window %f".to_string(),
            view_cmd: None,
        };
        let path = PathBuf::from("/tmp/main.rs");
        let (prog, args) = rule.resolve_open_cmd(&path);
        assert_eq!(prog, "code");
        assert_eq!(
            args,
            vec!["--new-window".to_string(), "/tmp/main.rs".to_string()]
        );
    }

    #[test]
    fn test_resolve_quoted_program_with_spaces() {
        let rule = AssocRule {
            mask: "*.txt".to_string(),
            open_cmd: r#""C:\Program Files\Editor\edit.exe" --wait %f"#.to_string(),
            view_cmd: None,
        };
        let path = PathBuf::from(r"C:\docs\My File.txt");
        let (prog, args) = rule.resolve_open_cmd(&path);
        assert_eq!(prog, r"C:\Program Files\Editor\edit.exe");
        assert_eq!(
            args,
            vec!["--wait".to_string(), r"C:\docs\My File.txt".to_string()]
        );
    }

    #[test]
    fn test_resolve_open_cmd_injection_neutralised() {
        let rule = AssocRule {
            mask: "*.txt".to_string(),
            open_cmd: "notepad %f".to_string(),
            view_cmd: None,
        };
        let path = PathBuf::from("/tmp/evil; rm -rf ~ #.txt");
        let (prog, args) = rule.resolve_open_cmd(&path);
        assert_eq!(prog, "notepad");
        assert_eq!(args, vec!["/tmp/evil; rm -rf ~ #.txt".to_string()]);
    }

    #[test]
    fn test_find_rule() {
        let config = AssociationsConfig::default_rules();
        let rule = config.find_rule("photo.png");
        assert!(rule.is_some());
        assert_eq!(rule.unwrap().mask, "*.{jpg,jpeg,png,gif,bmp,svg,webp}");
        assert!(config.find_rule("Cargo.toml").is_none());
    }

    #[test]
    fn migrate_drops_legacy_editor_rules_only() {
        let mut config = AssociationsConfig {
            rules: vec![
                AssocRule {
                    mask: "*.rs".to_string(),
                    open_cmd: "notepad %f".to_string(),
                    view_cmd: None,
                },
                AssocRule {
                    mask: "*.py".to_string(),
                    open_cmd: "code %f".to_string(),
                    view_cmd: None,
                },
            ],
        };
        assert!(config.migrate());
        assert_eq!(config.rules.len(), 1);
        assert_eq!(config.rules[0].mask, "*.py");
        assert!(!config.migrate());
    }

    #[test]
    fn test_roundtrip_serialization() {
        let config = AssociationsConfig::default_rules();
        let serialized = toml::to_string_pretty(&config).unwrap();
        let deserialized: AssociationsConfig = toml::from_str(&serialized).unwrap();
        assert_eq!(deserialized.rules.len(), config.rules.len());
    }
}
