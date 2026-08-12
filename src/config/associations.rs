use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use super::write_atomic;

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

    /// Returns the resolved open command as a `(program, args)` pair with the
    /// file path substituted for `%f`. The args are passed directly to the OS
    /// (via `Command::arg`) and never go through a shell, so a file path that
    /// contains shell metacharacters cannot trigger command injection.
    pub fn resolve_open_cmd(&self, path: &std::path::Path) -> (String, Vec<String>) {
        resolve_template(&self.open_cmd, path)
    }

    /// Returns the resolved view command as a `(program, args)` pair.
    /// Falls back to `open_cmd` if `view_cmd` is not set.
    pub fn resolve_view_cmd(&self, path: &std::path::Path) -> (String, Vec<String>) {
        let template = self.view_cmd.as_deref().unwrap_or(&self.open_cmd);
        resolve_template(template, path)
    }
}

/// Splits a command template into `(program, args)` and substitutes `%f` with
/// the file path. Whitespace separates tokens, but the path is delivered as
/// a single argument regardless of any whitespace it contains, so a file name
/// with spaces (e.g. `My File.txt`) does not get split into multiple argv
/// entries.
fn resolve_template(template: &str, path: &std::path::Path) -> (String, Vec<String>) {
    let path_str = path.to_string_lossy().into_owned();
    // We use a sentinel that cannot appear in a user-authored command
    // template (the NUL byte is not a valid character in a Windows or
    // Unix path and is not a meaningful token in a shell command). We
    // substitute it in for `%f` so the splitter never breaks the path
    // apart, then expand the sentinel back to the real path string.
    const SENTINEL: &str = "\u{1f}PaireeFileSentinel\u{1f}";
    let substituted = template.replace("%f", SENTINEL);
    let mut parts = substituted.split_whitespace();
    let program = parts.next().unwrap_or("").to_string();
    let args = parts
        .map(|s| {
            if s == SENTINEL {
                path_str.clone()
            } else {
                s.to_string()
            }
        })
        .collect();
    (program, args)
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
            Ok(mut config) => {
                let is_old = config.rules.len() == 4
                    && config.rules[0].mask == "*.rs"
                    && config.rules[1].mask == "*.toml"
                    && config.rules[2].mask == "*.md"
                    && config.rules[3].mask == "*.{zip,tar,gz,bz2,xz,7z}";
                if is_old {
                    config = Self::default_rules();
                    if let Err(e) = config.save() {
                        log::warn!("Failed to refresh associations.toml: {}", e);
                    }
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

    /// Finds the first rule whose mask matches the given filename.
    pub fn find_rule(&self, filename: &str) -> Option<&AssocRule> {
        self.rules.iter().find(|r| r.matches(filename))
    }

    /// Returns a default set of common rules for a fresh install.
    pub fn default_rules() -> Self {
        if cfg!(target_os = "windows") {
            Self {
                rules: vec![
                    AssocRule {
                        mask: "*.rs".to_string(),
                        open_cmd: "notepad %f".to_string(),
                        view_cmd: None,
                    },
                    AssocRule {
                        mask: "*.toml".to_string(),
                        open_cmd: "notepad %f".to_string(),
                        view_cmd: None,
                    },
                    AssocRule {
                        mask: "*.md".to_string(),
                        open_cmd: "notepad %f".to_string(),
                        view_cmd: None,
                    },
                    AssocRule {
                        mask: "*.{txt,json,yaml,yml,xml,ini,conf,cfg}".to_string(),
                        open_cmd: "notepad %f".to_string(),
                        view_cmd: None,
                    },
                    AssocRule {
                        mask: "*.{sh,bat,cmd,ps1,py,pl,rb,js,ts}".to_string(),
                        open_cmd: "notepad %f".to_string(),
                        view_cmd: None,
                    },
                    AssocRule {
                        mask: "*.{zip,tar,gz,bz2,xz,7z}".to_string(),
                        open_cmd: "explorer %f".to_string(),
                        view_cmd: None,
                    },
                    AssocRule {
                        mask: "*.{jpg,jpeg,png,gif,bmp,svg,webp}".to_string(),
                        open_cmd: "explorer %f".to_string(),
                        view_cmd: None,
                    },
                    AssocRule {
                        mask: "*.{mp3,wav,ogg,flac,m4a,mp4,mkv,avi,mov,wmv,webm}".to_string(),
                        open_cmd: "explorer %f".to_string(),
                        view_cmd: None,
                    },
                    AssocRule {
                        mask: "*.{pdf,doc,docx,xls,xlsx,ppt,pptx}".to_string(),
                        open_cmd: "explorer %f".to_string(),
                        view_cmd: None,
                    },
                    AssocRule {
                        mask: "*.{html,htm}".to_string(),
                        open_cmd: "explorer %f".to_string(),
                        view_cmd: None,
                    },
                ],
            }
        } else {
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
                        mask: "*.{txt,json,yaml,yml,xml,ini,conf,cfg}".to_string(),
                        open_cmd: "nano %f".to_string(),
                        view_cmd: Some("less %f".to_string()),
                    },
                    AssocRule {
                        mask: "*.{sh,bat,cmd,ps1,py,pl,rb,js,ts}".to_string(),
                        open_cmd: "nano %f".to_string(),
                        view_cmd: Some("less %f".to_string()),
                    },
                    AssocRule {
                        mask: "*.{zip,tar,gz,bz2,xz,7z}".to_string(),
                        open_cmd: "xdg-open %f".to_string(),
                        view_cmd: None,
                    },
                    AssocRule {
                        mask: "*.{jpg,jpeg,png,gif,bmp,svg,webp}".to_string(),
                        open_cmd: "xdg-open %f".to_string(),
                        view_cmd: None,
                    },
                    AssocRule {
                        mask: "*.{mp3,wav,ogg,flac,m4a,mp4,mkv,avi,mov,wmv,webm}".to_string(),
                        open_cmd: "xdg-open %f".to_string(),
                        view_cmd: None,
                    },
                    AssocRule {
                        mask: "*.{pdf,doc,docx,xls,xlsx,ppt,pptx}".to_string(),
                        open_cmd: "xdg-open %f".to_string(),
                        view_cmd: None,
                    },
                    AssocRule {
                        mask: "*.{html,htm}".to_string(),
                        open_cmd: "xdg-open %f".to_string(),
                        view_cmd: None,
                    },
                ],
            }
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
    fn test_resolve_open_cmd_injection_neutralised() {
        // A file name that looks like a shell-injection payload should be
        // passed verbatim to the program as a single argument, never to a shell.
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
