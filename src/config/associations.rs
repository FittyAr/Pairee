use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// A single file association rule: maps a glob mask to open/view commands.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssocRule {
    /// Glob mask, e.g. "*.rs" or "*.{jpg,png}"
    pub mask: String,
    /// Shell command to open the file (replaces `%f` with the file path).
    /// Example: "code %f"
    pub open_cmd: String,
    /// Optional viewer command for F3 (replaces `%f`). Falls back to open_cmd if None.
    pub view_cmd: Option<String>,
}

impl AssocRule {
    /// Returns true if the given filename matches this rule's mask.
    pub fn matches(&self, filename: &str) -> bool {
        crate::app::state::glob_matches(&self.mask, filename)
    }

    /// Returns the resolved open command with `%f` substituted by the file path.
    pub fn resolve_open_cmd(&self, path: &std::path::Path) -> String {
        self.open_cmd
            .replace("%f", &path.to_string_lossy())
    }

    /// Returns the resolved view command with `%f` substituted by the file path.
    pub fn resolve_view_cmd(&self, path: &std::path::Path) -> String {
        let cmd = self.view_cmd.as_deref().unwrap_or(&self.open_cmd);
        cmd.replace("%f", &path.to_string_lossy())
    }
}

/// Holds all file association rules. Loaded from / saved to `associations.toml`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AssociationsConfig {
    pub rules: Vec<AssocRule>,
}

impl AssociationsConfig {
    /// Loads associations from disk; returns an empty config if the file is missing.
    pub fn load() -> Self {
        match Self::try_load() {
            Ok(config) => config,
            Err(_) => Self::default(),
        }
    }

    fn try_load() -> Result<Self> {
        let path = associations_path();
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("Reading associations file {:?}", path))?;
        toml::from_str(&content).context("Deserializing associations.toml")
    }

    /// Persists the configuration to `<config_dir>/ncrust/associations.toml`.
    pub fn save(&self) -> Result<()> {
        let path = associations_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).context("Creating config directory")?;
        }
        let toml_str = toml::to_string_pretty(self).context("Serializing associations")?;
        std::fs::write(&path, toml_str)
            .with_context(|| format!("Writing associations file {:?}", path))
    }

    /// Finds the first rule whose mask matches the given filename.
    pub fn find_rule(&self, filename: &str) -> Option<&AssocRule> {
        self.rules.iter().find(|r| r.matches(filename))
    }

    /// Returns a default set of common rules for a fresh install.
    pub fn default_rules() -> Self {
        Self {
            rules: vec![
                AssocRule {
                    mask: "*.rs".to_string(),
                    open_cmd: "nano %f".to_string(),
                    view_cmd: Some("less %f".to_string()),
                },
                AssocRule {
                    mask: "*.toml".to_string(),
                    open_cmd: "nano %f".to_string(),
                    view_cmd: Some("less %f".to_string()),
                },
                AssocRule {
                    mask: "*.md".to_string(),
                    open_cmd: "nano %f".to_string(),
                    view_cmd: Some("less %f".to_string()),
                },
                AssocRule {
                    mask: "*.{zip,tar,gz,bz2,xz,7z}".to_string(),
                    open_cmd: "xdg-open %f".to_string(),
                    view_cmd: None,
                },
            ],
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
        assert_eq!(rule.resolve_open_cmd(&path), "nano /home/user/README.md");
    }

    #[test]
    fn test_find_rule() {
        let config = AssociationsConfig::default_rules();
        let rule = config.find_rule("Cargo.toml");
        assert!(rule.is_some());
        assert_eq!(rule.unwrap().mask, "*.toml");
    }

    #[test]
    fn test_roundtrip_serialization() {
        let config = AssociationsConfig::default_rules();
        let serialized = toml::to_string_pretty(&config).unwrap();
        let deserialized: AssociationsConfig = toml::from_str(&serialized).unwrap();
        assert_eq!(deserialized.rules.len(), config.rules.len());
    }
}
