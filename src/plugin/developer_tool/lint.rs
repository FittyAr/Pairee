use super::progress_status;
use crate::app::state::DevProgress;
use crate::config::localization::t;
use tokio::sync::mpsc::UnboundedSender;

pub fn lint() -> anyhow::Result<()> {
    lint_with_progress(None)
}

pub fn lint_with_progress(progress: Option<UnboundedSender<DevProgress>>) -> anyhow::Result<()> {
    let path = std::env::current_dir()?;
    progress_status(&progress, t("plugin_dev_progress_reading_manifest"));
    let manifest_path = path.join("manifest.toml");
    if !manifest_path.exists() {
        anyhow::bail!(t("plugin_dev_lint_err_manifest").trim().to_string());
    }
    let content = std::fs::read_to_string(&manifest_path)?;

    let manifest = crate::plugin::loader::PluginManifest::parse(&content)?;

    if manifest
        .default_language
        .as_ref()
        .is_none_or(|l| l.trim().is_empty())
    {
        anyhow::bail!(t("plugin_dev_lint_err_default_lang"));
    }

    print!(
        "{}",
        t("plugin_dev_lint_start").replace("{}", &manifest.name)
    );

    progress_status(&progress, t("plugin_dev_progress_checking_lua"));
    let main_path = path.join("main.lua");
    if !main_path.exists() {
        anyhow::bail!(t("plugin_dev_lint_err_lua").trim().to_string());
    }
    let lua_code = std::fs::read_to_string(&main_path)?;

    // Basic forbidden pattern linting
    let mut warnings = 0;
    if !manifest.requires_trust.unwrap_or(false) {
        let forbidden = ["os.execute", "io.open", "os.system", "dofile", "loadfile"];
        for f in &forbidden {
            if lua_code.contains(f) {
                print!("{}", t("plugin_dev_lint_warn_unsafe").replace("{}", f));
                warnings += 1;
            }
        }
    }

    for problem in key_problems(&manifest) {
        print!("{}", t("plugin_dev_lint_warn_key").replace("{}", &problem));
        warnings += 1;
    }

    if warnings == 0 {
        print!("{}", t("plugin_dev_lint_ok"));
        println!();
    } else {
        print!(
            "{}",
            t("plugin_dev_lint_warn_total").replace("{}", &warnings.to_string())
        );
        println!();
    }
    Ok(())
}

/// Key problems of the plugin's commands in each built-in preset: invalid
/// chords, and suggested keys that preset already uses (they would stay
/// unbound there), as `[preset] detail`.
pub fn key_problems(manifest: &crate::plugin::loader::PluginManifest) -> Vec<String> {
    use crate::keybindings::embedded::PRESETS;
    use crate::keybindings::loader::{KeymapSpec, build_shipped_keymap};
    use crate::keybindings::plugin_commands;
    plugin_commands::register(&manifest.name, manifest.keymap_commands());
    let plugins: Vec<_> = plugin_commands::active()
        .into_iter()
        .filter(|(_, c)| c.plugin == manifest.name)
        .collect();
    let keybindings = crate::config::keybindings::KeybindingsConfig::default();
    let prefix = format!("plugin.{}.", manifest.name);
    let mut out = Vec::new();
    for (preset, _) in PRESETS {
        let map = build_shipped_keymap(&KeymapSpec {
            preset,
            keybindings: &keybindings,
            yazi_letters: false,
            plugins: plugins.clone(),
        });
        let report = map.report;
        out.extend(
            report
                .errors
                .iter()
                .chain(&report.conflicts)
                .filter(|p| p.contains(&prefix))
                .map(|p| format!("[{preset}] {p}")),
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suggested_keys_taken_by_a_preset_are_reported_per_preset() {
        let manifest = crate::plugin::loader::PluginManifest::parse(
            "name = \"lint-demo\"
version = \"1\"
             [[commands]]
id = \"go\"
[commands.keys]
default = \"F5\"
neovim = \"<leader>pg\"
",
        )
        .unwrap();
        let problems = key_problems(&manifest);
        assert!(
            problems
                .iter()
                .any(|p| p.starts_with("[norton]") && p.contains("'F5'"))
        );
        assert!(
            !problems.iter().any(|p| p.starts_with("[neovim]")),
            "{problems:?}"
        );
    }
}
