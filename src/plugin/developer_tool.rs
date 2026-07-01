use crate::config::localization::t;
use std::collections::HashMap;

const TEMPLATE_BRANCH: &str = "plugin-template";

/// Locates the Pairee git repository on the local file system.
///
/// Search order:
/// 1. `PAIREE_REPO_DIR` environment variable (explicit override).
/// 2. Walk up from the running binary until a `.git` directory is found.
fn find_pairee_repo() -> Option<std::path::PathBuf> {
    // 1. Explicit env override
    if let Ok(dir) = std::env::var("PAIREE_REPO_DIR") {
        let p = std::path::PathBuf::from(dir);
        if p.join(".git").exists() {
            return Some(p);
        }
    }

    // 2. Walk up from binary location
    if let Ok(exe) = std::env::current_exe() {
        let mut candidate = exe.parent()?.to_path_buf();
        loop {
            if candidate.join(".git").exists() {
                return Some(candidate);
            }
            match candidate.parent() {
                Some(p) => candidate = p.to_path_buf(),
                None => break,
            }
        }
    }

    None
}

/// Copies all files from the `plugin-template` git branch into `target_path`
/// using the `git2` crate (no external `git` binary required).
///
/// After copying, replaces the placeholders `PLUGIN_NAME`, `PLUGIN_DESCRIPTION`
/// and `PLUGIN_AUTHOR` inside `manifest.toml` and `help/en.md`.
///
/// Returns `true` if the template was found and copied successfully, or `false`
/// if the repo/branch is unavailable (triggering the caller to fall back to the
/// localization-string method).
fn clone_from_template(
    target_path: &std::path::Path,
    manifest_name: &str,
    description: &str,
    author: &str,
) -> anyhow::Result<bool> {
    let Some(repo_dir) = find_pairee_repo() else {
        log::debug!("plugin-template: Pairee repo not found; using fallback.");
        return Ok(false);
    };

    let repo = match git2::Repository::open(&repo_dir) {
        Ok(r) => r,
        Err(e) => {
            log::debug!("plugin-template: Could not open repo: {}", e);
            return Ok(false);
        }
    };

    let branch_ref = format!("refs/heads/{}", TEMPLATE_BRANCH);
    let reference = match repo.find_reference(&branch_ref) {
        Ok(r) => r,
        Err(_) => {
            log::debug!(
                "plugin-template: Branch '{}' not found; using fallback.",
                TEMPLATE_BRANCH
            );
            return Ok(false);
        }
    };

    let commit = reference.peel_to_commit()?;
    let tree = commit.tree()?;

    // Walk every blob in the tree and write it to target_path
    tree.walk(git2::TreeWalkMode::PreOrder, |root, entry| {
        use git2::ObjectType;
        let rel_path = if root.is_empty() {
            entry.name().unwrap_or("").to_string()
        } else {
            format!("{}{}", root, entry.name().unwrap_or(""))
        };

        match entry.kind() {
            Some(ObjectType::Blob) => {
                let obj = match entry.to_object(&repo) {
                    Ok(o) => o,
                    Err(_) => return git2::TreeWalkResult::Ok,
                };
                let blob = match obj.as_blob() {
                    Some(b) => b,
                    None => return git2::TreeWalkResult::Ok,
                };
                let dest = target_path.join(&rel_path);
                if let Some(parent) = dest.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                let _ = std::fs::write(&dest, blob.content());
            }
            Some(ObjectType::Tree) => {
                let dir = target_path.join(&rel_path);
                let _ = std::fs::create_dir_all(&dir);
            }
            _ => {}
        }
        git2::TreeWalkResult::Ok
    })?;

    // Replace placeholders in manifest.toml
    let manifest_path = target_path.join("manifest.toml");
    if manifest_path.exists() {
        let content = std::fs::read_to_string(&manifest_path)?;
        let content = content
            .replace("PLUGIN_NAME", manifest_name)
            .replace("PLUGIN_DESCRIPTION", description)
            .replace("PLUGIN_AUTHOR", author);
        std::fs::write(&manifest_path, content)?;
    }

    // Replace placeholder in help/en.md
    let help_path = target_path.join("help").join("en.md");
    if help_path.exists() {
        let content = std::fs::read_to_string(&help_path)?;
        let content = content.replace("PLUGIN_NAME", manifest_name);
        std::fs::write(&help_path, content)?;
    }

    log::info!(
        "plugin-template: Plugin '{}' initialized from git branch '{}'.",
        manifest_name,
        TEMPLATE_BRANCH
    );
    Ok(true)
}

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

    // --- Primary: clone files from the `plugin-template` git branch -----------
    let used_template = clone_from_template(&path, &manifest_name, description, author)?;

    // --- Fallback: generate files from localization strings -------------------
    if !used_template {
        log::warn!(
            "plugin-template: Template branch unavailable; falling back to localization strings."
        );

        std::fs::create_dir_all(path.join("lang"))?;
        std::fs::create_dir_all(path.join("help"))?;
        std::fs::create_dir_all(path.join("screenshots"))?;

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

        let help_en = t("plugin_init_help_en_tmpl").replace("{}", &manifest_name);
        std::fs::write(path.join("help").join("en.md"), help_en)?;

        // Generate placeholder PNG images
        let icon_img = image::RgbImage::new(256, 256);
        icon_img.save(path.join("icon.png"))?;

        let screenshot_img = image::RgbImage::new(640, 480);
        screenshot_img.save(path.join("screenshots").join("screenshot1.png"))?;
    }

    // If Spanish is active, generate es.toml and es.md regardless of source
    if let Ok(config) = crate::config::AppConfig::load_or_create() {
        let current_lang = config.settings.language.to_lowercase();
        if current_lang.contains("spanish") || current_lang.contains("es") {
            std::fs::create_dir_all(path.join("lang"))?;
            std::fs::create_dir_all(path.join("help"))?;

            let lang_es = t("plugin_init_lang_es_tmpl");
            std::fs::write(path.join("lang").join("es.toml"), lang_es)?;

            let help_es = t("plugin_init_help_es_tmpl").replace("{}", &manifest_name);
            std::fs::write(path.join("help").join("es.md"), help_es)?;
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

    if manifest.default_language.as_ref().map_or(true, |l| l.trim().is_empty()) {
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

    let content = match std::fs::read_to_string(&manifest_path) {
        Ok(c) => c,
        Err(e) => return Err(format!("Error reading manifest.toml: {:?}", e)),
    };

    let manifest = match crate::plugin::loader::PluginManifest::parse(&content) {
        Ok(m) => m,
        Err(e) => return Err(format!("Error parsing manifest.toml: {:?}", e)),
    };

    // 1. Validate Icon
    let icon_rel = match &manifest.icon {
        Some(i) if !i.trim().is_empty() => i.trim(),
        _ => return Err(t("plugin_dev_publish_no_icon")),
    };
    let icon_path = path.join(icon_rel);
    if !icon_path.exists() || !icon_path.is_file() {
        return Err(t("plugin_dev_publish_no_icon"));
    }

    // Check dimensions: 256x256 or 512x512
    match image::image_dimensions(&icon_path) {
        Ok((w, h)) => {
            if (w != 256 || h != 256) && (w != 512 || h != 512) {
                return Err(t("plugin_dev_publish_icon_invalid_size")
                    .replace("{w}", &w.to_string())
                    .replace("{h}", &h.to_string()));
            }
        }
        Err(e) => {
            return Err(t("plugin_dev_publish_icon_invalid_format")
                .replace("{:?}", &format!("{:?}", e)));
        }
    }

    // 2. Validate Screenshots
    let screenshots = match &manifest.screenshots {
        Some(s) if !s.is_empty() => s,
        _ => return Err(t("plugin_dev_publish_no_screenshots")),
    };

    for scr_rel in screenshots {
        if scr_rel.trim().is_empty() {
            continue;
        }
        let scr_path = path.join(scr_rel);
        if !scr_path.exists() || !scr_path.is_file() {
            return Err(t("plugin_dev_publish_screenshot_not_found").replace("{}", scr_rel));
        }

        // Validate screenshot size: minimum 640x480 pixels
        match image::image_dimensions(&scr_path) {
            Ok((w, h)) => {
                if w < 640 || h < 480 {
                    return Err(t("plugin_dev_publish_screenshot_invalid_size")
                        .replace("{}", scr_rel)
                        .replace("{w}", &w.to_string())
                        .replace("{h}", &h.to_string()));
                }
            }
            Err(e) => {
                return Err(t("plugin_dev_publish_screenshot_invalid_format")
                    .replace("{}", scr_rel)
                    .replace("{:?}", &format!("{:?}", e)));
            }
        }
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
