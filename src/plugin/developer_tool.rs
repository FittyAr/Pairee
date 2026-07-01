use crate::config::localization::t;
use std::collections::HashMap;

pub fn init(name: &str) -> anyhow::Result<()> {
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
    let manifest = manifest_tmpl.replace("{}", &manifest_name);
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

    println!("Initialized plugin boilerplate in {:?}", path);
    Ok(())
}

pub fn lint() -> anyhow::Result<()> {
    let path = std::env::current_dir()?;
    let manifest_path = path.join("manifest.toml");
    if !manifest_path.exists() {
        anyhow::bail!("Error: manifest.toml not found in current directory");
    }
    let content = std::fs::read_to_string(&manifest_path)?;

    // Parse under [plugin] section or top-level depending on format
    // Let's allow parsing easily
    let manifest = crate::plugin::loader::PluginManifest::parse(&content)?;

    println!("Linting plugin '{}'...", manifest.name);

    let main_path = path.join("main.lua");
    if !main_path.exists() {
        anyhow::bail!("Error: main.lua not found in current directory");
    }
    let lua_code = std::fs::read_to_string(&main_path)?;

    // Basic forbidden pattern linting
    let mut warnings = 0;
    if !manifest.requires_trust.unwrap_or(false) {
        let forbidden = ["os.execute", "io.open", "os.system", "dofile", "loadfile"];
        for f in &forbidden {
            if lua_code.contains(f) {
                println!(
                    "  [Warning] Un-trusted plugin uses potentially unsafe method '{}'",
                    f
                );
                warnings += 1;
            }
        }
    }

    if warnings == 0 {
        println!("✓ Lint passed cleanly!");
    } else {
        println!("Lint completed with {} warnings.", warnings);
    }
    Ok(())
}

pub fn package() -> anyhow::Result<()> {
    let path = std::env::current_dir()?;
    let manifest_path = path.join("manifest.toml");
    if !manifest_path.exists() {
        anyhow::bail!("Error: manifest.toml not found");
    }
    let content = std::fs::read_to_string(&manifest_path)?;

    let manifest = crate::plugin::loader::PluginManifest::parse(&content)?;

    println!("Packaging plugin '{}'...", manifest.name);

    let mut files_hash = HashMap::new();

    // Iterate files in plugin directory dynamically
    for (file_rel, file_path) in crate::plugin::loader::get_plugin_files(&path) {
        let hash = crate::update::downloader::compute_sha256(&file_path)?;
        files_hash.insert(file_rel, hash);
    }

    // Output TOML registry entry
    println!("\nGenerated registry entry to append to registry/index.toml:\n");
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

pub async fn submit() -> anyhow::Result<()> {
    println!("GitHub PR Submission Wizard");
    println!("---------------------------");

    // Read input from user for GitHub token
    println!("Please enter your GitHub Personal Access Token:");
    let mut token = String::new();
    std::io::stdin().read_line(&mut token)?;
    let token = token.trim().to_string();

    if token.is_empty() {
        anyhow::bail!("GitHub Token is required to submit a Pull Request.");
    }

    let path = std::env::current_dir()?;
    let manifest_path = path.join("manifest.toml");
    if !manifest_path.exists() {
        anyhow::bail!(
            "No manifest.toml found in current directory. Move to your plugin directory before submitting."
        );
    }

    println!("✓ Token received. Ready to fork and submit.");
    println!("Sending submit request to Pairee's main repository registry...");

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
        println!("✓ Upstream repository forked successfully.");
    } else {
        anyhow::bail!("Failed to fork upstream repository: HTTP {}", resp.status());
    }

    // Since this is a CLI helper, we inform the developer of next steps or complete the commit/push
    println!("Please run the git push commands to update your branch on the fork,");
    println!("then submit the Pull Request using the GitHub API or UI.");
    Ok(())
}
