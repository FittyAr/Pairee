//! Markdown reference of a shipped preset: its options and every bound
//! command of each context, by category. The F1 help pages
//! `help/<lang>/keymap_<preset>.md` are generated with it (a test keeps them
//! in sync) and `pairee keymap print` prints it.

use super::actions::Action;
use super::catalog::Category;
use super::keymap::ContextKeymap;
use super::loader::{KeymapSpec, LoadedKeymap, build_shipped_keymap};
use super::options::{KeymapOptions, TypingMode};
use super::registry::Bindable;
use crate::config::keybindings::KeybindingsConfig;

/// Translates an i18n key.
pub type Translate<'a> = &'a dyn Fn(&str) -> String;

/// The reference of shipped preset `preset`, translated by `tr`.
pub fn markdown(preset: &str, tr: Translate) -> String {
    let keybindings = KeybindingsConfig::default();
    let loaded = build_shipped_keymap(&KeymapSpec {
        preset,
        keybindings: &keybindings,
        yazi_letters: false,
        plugins: Vec::new(),
    });
    let mut out = header(preset, &loaded, tr);
    out.push_str(&section(tr("shortcuts_tab_panels"), &loaded.panels, tr));
    out.push_str(&section(tr("shortcuts_tab_editor"), &loaded.editor, tr));
    out.push_str(&section(tr("shortcuts_tab_viewer"), &loaded.viewer, tr));
    out.push_str(&section(tr("shortcuts_tab_list"), &loaded.list, tr));
    out
}

fn header(preset: &str, loaded: &LoadedKeymap, tr: Translate) -> String {
    let name = tr(&format!("onboarding_{preset}"));
    let modal = loaded
        .panels
        .key_for(Action::WhichKey)
        .map(code)
        .unwrap_or_default();
    format!(
        "# {}\n\n{}\n\n{}",
        tr("keymap_doc_title").replace("{}", &name),
        tr("keymap_doc_intro").replace("{}", &modal),
        options(&loaded.options, tr)
    )
}

fn options(options: &KeymapOptions, tr: Translate) -> String {
    let typing = match options.typing {
        TypingMode::Cli => "keymap_doc_typing_cli",
        TypingMode::TypeAhead => "keymap_doc_typing_type_ahead",
        TypingMode::Commands => "keymap_doc_typing_commands",
    };
    let mut lines = vec![format!("- {}", tr(typing))];
    if options.alt_quick_search {
        lines.push(format!("- {}", tr("keymap_doc_alt_search")));
    }
    if let Some(leader) = &options.leader {
        lines.push(format!(
            "- {}",
            tr("keymap_doc_leader").replace("{}", &code(leader))
        ));
    }
    if options.sequence_timeout_ms == 0 {
        lines.push(format!("- {}", tr("keymap_doc_wait")));
    }
    lines.join("\n") + "\n"
}

/// One `##` section: a `###` table per category of the bound commands.
fn section<B: Bindable>(title: String, keymap: &ContextKeymap<B>, tr: Translate) -> String {
    let mut commands: Vec<(Category, String, B)> = B::all()
        .into_iter()
        .filter(|c| !keymap.keys_for(*c).is_empty())
        .map(|c| (c.category(), c.label_in(tr), c))
        .collect();
    commands.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
    let mut out = format!("\n## {title}\n");
    let mut current: Option<Category> = None;
    for (category, label, command) in commands {
        if current != Some(category) {
            current = Some(category);
            out.push_str(&format!(
                "\n### {}\n\n| {} | {} |\n| :--- | :--- |\n",
                tr(category.label_key()),
                tr("keymap_doc_keys"),
                tr("keymap_doc_action")
            ));
        }
        let keys: Vec<String> = keymap.keys_for(command).iter().map(|k| code(k)).collect();
        out.push_str(&format!("| {} | {label} |\n", keys.join(" · ")));
    }
    out
}

/// `text` as inline code, safe for backticks and table pipes.
fn code(text: &str) -> String {
    let text = text.replace('|', "\\|");
    if text.contains('`') {
        format!("`` {text} ``")
    } else {
        format!("`{text}`")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::localization::translator::get_default_english_translation;
    use crate::keybindings::embedded::PRESETS;
    use std::collections::HashMap;
    use std::path::{Path, PathBuf};

    fn repo() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    /// Translations of `lang/<code>.toml`, English for missing keys.
    fn translator(code: &str) -> impl Fn(&str) -> String {
        let text = std::fs::read_to_string(repo().join(format!("lang/{code}.toml"))).unwrap();
        let file: crate::config::localization::types::LanguageFile = toml::from_str(&text).unwrap();
        let table: HashMap<String, String> = file.translations;
        move |key: &str| {
            table
                .get(key)
                .cloned()
                .unwrap_or_else(|| get_default_english_translation(key))
        }
    }

    fn page(code: &str, preset: &str) -> PathBuf {
        repo().join(format!("help/{code}/keymap_{preset}.md"))
    }

    fn read(path: &Path) -> String {
        std::fs::read_to_string(path)
            .unwrap_or_default()
            .replace("\r\n", "\n")
    }

    /// The help pages match the presets. `PAIREE_REGEN_DOCS=1 cargo test
    /// keymap_reference` rewrites them after a preset change.
    #[test]
    fn keymap_reference_pages_match_the_presets() {
        let regen = std::env::var("PAIREE_REGEN_DOCS").is_ok();
        let mut stale = Vec::new();
        for code in ["en", "es"] {
            let tr = translator(code);
            for (preset, _) in PRESETS {
                let text = markdown(preset, &tr);
                let path = page(code, preset);
                if regen {
                    std::fs::write(&path, &text).unwrap();
                } else if read(&path) != text {
                    stale.push(path.display().to_string());
                }
            }
        }
        assert!(
            stale.is_empty(),
            "outdated (run PAIREE_REGEN_DOCS=1 cargo test keymap_reference): {stale:?}"
        );
    }

    #[test]
    fn reference_lists_options_and_keys() {
        let text = markdown("neovim", &get_default_english_translation);
        assert!(text.contains("| `y y` |"), "{text}");
        assert!(text.contains("`Space`"), "leader is listed");
        assert_eq!(code("Ctrl+`"), "`` Ctrl+` ``");
    }
}
