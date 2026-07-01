use std::collections::HashMap;

pub fn init(name: &str) -> anyhow::Result<()> {
    let path = std::env::current_dir()?.join(name);
    std::fs::create_dir_all(&path)?;
    std::fs::create_dir_all(path.join("lang"))?;

    let manifest = format!(
        r#"[plugin]
name = "{}"
version = "0.1.0"
description = "A new plugin for Pairee"
author = "Your Name"
min_pairee = "0.6.1"
requires_trust = false
default_language = "en"
languages = ["en"]

[keybindings]
# "ctrl-p" = "my_custom_action"

[settings_schema]
# enabled = {{ type = "boolean", default = true, description = "Enable features" }}
"#,
        name
    );
    std::fs::write(path.join("manifest.toml"), manifest)?;

    let main_lua = r#"-- Pairee Plugin Entry
local plugin = {}

function plugin.setup(opts)
    pairee.log.info("Hello from my new plugin!")
end

-- Custom Command Entry
function plugin.entry(args)
    pairee.app.notify("My Plugin", "Executed command with " .. #args .. " args", "info")
end

-- Custom Previewer
function plugin.peek(job)
    return pairee.ui.Paragraph("Previewing file: " .. job.file.path)
end

return plugin
"#;
    std::fs::write(path.join("main.lua"), main_lua)?;

    let lang_en = r#"[my_custom_action]
title = "My Custom Action"
description = "Executes my custom action"
"#;
    std::fs::write(path.join("lang").join("en.toml"), lang_en)?;

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

    // Iterate files in plugin directory
    let files_to_hash = ["manifest.toml", "main.lua", "lang/en.toml"];
    for file_rel in &files_to_hash {
        let file_path = path.join(file_rel);
        if file_path.exists() {
            let hash = crate::update::downloader::compute_sha256(&file_path)?;
            files_hash.insert(file_rel.to_string(), hash);
        }
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
