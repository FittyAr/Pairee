//! Default association rules shipped on a fresh install.
//!
//! Pairee edits text exclusively with its built-in editor (F4), so the
//! defaults only hand non-text formats (archives, media, documents) to the
//! operating system's opener. Text files have no rule and therefore open in
//! the internal viewer/editor.

use super::rule::AssocRule;

/// Masks handed to the system opener by default.
const SYSTEM_OPENER_MASKS: &[&str] = &[
    "*.{zip,tar,gz,bz2,xz,7z}",
    "*.{jpg,jpeg,png,gif,bmp,svg,webp}",
    "*.{mp3,wav,ogg,flac,m4a,mp4,mkv,avi,mov,wmv,webm}",
    "*.{pdf,doc,docx,xls,xlsx,ppt,pptx}",
    "*.{html,htm}",
];

/// Text masks that older releases associated with an external editor.
const LEGACY_EDITOR_MASKS: &[&str] = &[
    "*.rs",
    "*.toml",
    "*.md",
    "*.{txt,json,yaml,yml,xml,ini,conf,cfg}",
    "*.{sh,bat,cmd,ps1,py,pl,rb,js,ts}",
];

/// Editor commands that older releases shipped for [`LEGACY_EDITOR_MASKS`].
const LEGACY_EDITOR_COMMANDS: &[&str] = &["notepad %f", "nano %f"];

fn system_opener_command() -> &'static str {
    if cfg!(target_os = "windows") {
        "explorer %f"
    } else {
        "xdg-open %f"
    }
}

pub fn get_default_rules() -> Vec<AssocRule> {
    SYSTEM_OPENER_MASKS
        .iter()
        .map(|mask| AssocRule {
            mask: (*mask).to_string(),
            open_cmd: system_opener_command().to_string(),
            view_cmd: None,
        })
        .collect()
}

/// `true` for an untouched default rule from an older release that launched
/// an external text editor. Such rules are dropped on load.
pub fn is_legacy_editor_rule(rule: &AssocRule) -> bool {
    LEGACY_EDITOR_MASKS.contains(&rule.mask.as_str())
        && LEGACY_EDITOR_COMMANDS.contains(&rule.open_cmd.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_never_launch_an_editor() {
        for rule in get_default_rules() {
            assert!(!is_legacy_editor_rule(&rule));
            assert!(!rule.matches("notes.txt"), "{} matches text", rule.mask);
        }
    }

    #[test]
    fn legacy_editor_rule_detected() {
        let rule = AssocRule {
            mask: "*.md".to_string(),
            open_cmd: "nano %f".to_string(),
            view_cmd: Some("less %f".to_string()),
        };
        assert!(is_legacy_editor_rule(&rule));
        let custom = AssocRule {
            open_cmd: "code %f".to_string(),
            ..rule
        };
        assert!(!is_legacy_editor_rule(&custom));
    }
}
