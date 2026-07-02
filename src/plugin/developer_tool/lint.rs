use super::progress_status;
use crate::app::state::DevProgress;
use crate::config::localization::t;
use tokio::sync::mpsc::UnboundedSender;

pub fn lint() -> anyhow::Result<()> {
    lint_with_progress(None)
}

pub fn lint_with_progress(
    progress: Option<UnboundedSender<DevProgress>>,
) -> anyhow::Result<()> {
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
        .map_or(true, |l| l.trim().is_empty())
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
