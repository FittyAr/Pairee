use crate::config::localization::t;
use std::collections::HashMap;

pub fn init(name: &str, description: &str, author: &str, print_output: bool) -> anyhow::Result<()> {
    let folder_name = if name.ends_with(".pairee") {
        name.to_string()
    } else {
        format!("{}.pairee", name)
    };
    let manifest_name = folder_name
        .strip_suffix(".pairee")
        .unwrap_or(&folder_name)
        .to_string();

    let path = std::env::current_dir()?.join(&folder_name);
    std::fs::create_dir_all(&path)?;
    std::fs::create_dir_all(path.join("lang"))?;

    let manifest_tmpl = t("plugin_init_manifest_tmpl");
    let mut manifest = String::new();
    for line in manifest_tmpl.lines() {
        if line.starts_with("name = ") {
            manifest.push_str(&format!("name = \"{}\"\n", manifest_name));
        } else if line.starts_with("description = ") {
            manifest.push_str(&format!("description = \"{}\"\n", description));
        } else if line.starts_with("author = ") {
            manifest.push_str(&format!("author = \"{}\"\n", author));
        } else {
            manifest.push_str(line);
            manifest.push_str("\n");
        }
    }
    std::fs::write(path.join("manifest.toml"), manifest)?;

    let main_lua = t("plugin_init_main_lua_tmpl");
    std::fs::write(path.join("main.lua"), main_lua)?;

    let lang_en = t("plugin_init_lang_en_tmpl");
    std::fs::write(path.join("lang").join("en.toml"), lang_en)?;

    // If Spanish is active, also generate es.toml as an example of localization
    if let Ok(config) = crate::config::AppConfig::load_or_create() {
        let current_lang = config.settings.language.to_lowercase();
        if current_lang.contains("spanish") || current_lang.contains("es") {
            let lang_es = t("plugin_init_lang_es_tmpl");
            std::fs::write(path.join("lang").join("es.toml"), lang_es)?;
        }
    }

    if print_output {
        let ok_msg = t("plugin_dev_init_ok")
            .replace("{}", &manifest_name)
            .replace("{:?}", &format!("{:?}", path));
        println!("{}", ok_msg);
    }
    Ok(())
}

pub fn lint() -> anyhow::Result<()> {
    let path = std::env::current_dir()?;
    let manifest_path = path.join("manifest.toml");
    if !manifest_path.exists() {
        anyhow::bail!(t("plugin_dev_lint_err_manifest").trim().to_string());
    }
    let content = std::fs::read_to_string(&manifest_path)?;

    // Parse under [plugin] section or top-level depending on format
    // Let's allow parsing easily
    let manifest = crate::plugin::loader::PluginManifest::parse(&content)?;

    if manifest.default_language.is_none()
        || manifest
            .default_language
            .as_ref()
            .unwrap()
            .trim()
            .is_empty()
    {
        anyhow::bail!(t("plugin_dev_lint_err_default_lang"));
    }

    print!(
        "{}",
        t("plugin_dev_lint_start").replace("{}", &manifest.name)
    );

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

pub fn package() -> anyhow::Result<()> {
    let path = std::env::current_dir()?;
    let manifest_path = path.join("manifest.toml");
    if !manifest_path.exists() {
        anyhow::bail!(t("plugin_dev_lint_err_manifest").trim().to_string());
    }
    let content = std::fs::read_to_string(&manifest_path)?;

    let manifest = crate::plugin::loader::PluginManifest::parse(&content)?;

    print!(
        "{}",
        t("plugin_dev_pack_start").replace("{}", &manifest.name)
    );

    let mut files_hash = HashMap::new();

    // Iterate files in plugin directory dynamically
    for (file_rel, file_path) in crate::plugin::loader::get_plugin_files(&path) {
        let hash = crate::update::downloader::compute_sha256(&file_path)?;
        files_hash.insert(file_rel, hash);
    }

    // Output TOML registry entry
    print!("{}", t("plugin_dev_pack_gen"));
    println!("[plugins.{}]", manifest.name);
    println!("name = \"{}\"", manifest.name);
    println!("version = \"{}\"", manifest.version);
    if let Some(ref d) = manifest.description {
        println!("description = \"{}\"", d);
    }
    if let Some(ref a) = manifest.author {
        println!("author = \"{}\"", a);
    }
    if let Some(ref mp) = manifest.min_pairee {
        println!("min_pairee = \"{}\"", mp);
    }
    println!("files = {{");
    for (f, h) in files_hash {
        println!("    \"{}\" = \"{}\",", f, h);
    }
    println!("}}");

    Ok(())
}

pub fn validate_for_publish(path: &std::path::Path) -> Result<(), String> {
    let manifest_path = path.join("manifest.toml");
    if !manifest_path.exists() {
        return Err(t("plugin_dev_submit_no_manifest"));
    }

    let icon_path = path.join("icon.png");
    if !icon_path.exists() {
        return Err(t("plugin_dev_publish_no_icon"));
    }

    let screenshots_dir = path.join("screenshots");
    if !screenshots_dir.exists() || !screenshots_dir.is_dir() {
        return Err(t("plugin_dev_publish_no_screenshots_dir"));
    }

    let mut has_screenshot = false;
    if let Ok(entries) = std::fs::read_dir(screenshots_dir) {
        for entry in entries.filter_map(Result::ok) {
            let p = entry.path();
            if p.is_file() {
                if let Some(ext) = p.extension().and_then(|e| e.to_str()) {
                    let ext_lower = ext.to_lowercase();
                    if ext_lower == "png" || ext_lower == "jpg" || ext_lower == "jpeg" {
                        has_screenshot = true;
                        break;
                    }
                }
            }
        }
    }

    if !has_screenshot {
        return Err(t("plugin_dev_publish_no_screenshots"));
    }

    Ok(())
}

pub async fn submit() -> anyhow::Result<()> {
    println!("{}", t("plugin_dev_submit_wizard"));

    let path = std::env::current_dir()?;
    if let Err(err_msg) = validate_for_publish(&path) {
        anyhow::bail!(err_msg);
    }

    // Read input from user for GitHub token
    println!("{}", t("plugin_dev_submit_prompt"));
    let mut token = String::new();
    std::io::stdin().read_line(&mut token)?;
    let token = token.trim().to_string();

    if token.is_empty() {
        anyhow::bail!(t("plugin_dev_submit_token_req"));
    }

    println!("{}", t("plugin_dev_submit_token_ok"));
    println!("{}", t("plugin_dev_submit_sending"));

    // Simulate/Perform GitHub fork and PR creation via REST API
    let client = reqwest::Client::builder().build()?;

    // 1. Fork repository
    let fork_url = "https://api.github.com/repos/FittyAr/Pairee/forks";
    let resp = client
        .post(fork_url)
        .header("User-Agent", "Pairee-Submit-Wizard")
        .header("Authorization", format!("token {}", token))
        .send()
        .await?;

    if resp.status().is_success() || resp.status() == reqwest::StatusCode::ACCEPTED {
        println!("{}", t("plugin_dev_submit_fork_ok"));
    } else {
        anyhow::bail!(t("plugin_dev_submit_fork_err").replace("{}", &resp.status().to_string()));
    }

    // Since this is a CLI helper, we inform the developer of next steps or complete the commit/push
    println!("{}", t("plugin_dev_submit_next_steps"));
    Ok(())
}
