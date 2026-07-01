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
    let msg = package_to_registry(&path)?;
    println!("{}", msg);
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

    let manifest_path = path.join("manifest.toml");
    let content = std::fs::read_to_string(&manifest_path)?;
    let manifest = crate::plugin::loader::PluginManifest::parse(&content)?;
    let plugin_name = manifest.name.clone();

    // 1. Run package_to_registry to make sure everything is in the temp registry clone
    let msg = package_to_registry(&path)?;
    println!("{}", msg);

    // 2. Ask user for commit description
    println!("Enter Commit Description / Message:");
    let mut commit_msg = String::new();
    std::io::stdin().read_line(&mut commit_msg)?;
    let commit_msg = commit_msg.trim().to_string();
    if commit_msg.is_empty() {
        anyhow::bail!("Commit message cannot be empty.");
    }

    // 3. Commit locally
    commit_registry_changes(&commit_msg)?;
    println!("Changes staged and committed to local registry branch.");

    // 4. Prompt for token (optional)
    println!("{}", t("plugin_dev_submit_prompt")); // "Enter GitHub Token: "
    let mut token = String::new();
    std::io::stdin().read_line(&mut token)?;
    let token = token.trim().to_string();

    if token.is_empty() {
        let temp_dir = crate::config::paths::get_cache_dir().join("temp_registry");
        println!("\nNo token provided. You must submit manually:");
        println!("1. Fork FittyAr/Pairee repository on GitHub.");
        println!("2. Run the following commands in your terminal:");
        println!("   cd \"{}\"", temp_dir.display());
        println!("   git remote add myfork <URL_TO_YOUR_FORK>");
        println!("   git push myfork plugin-registry");
        println!("3. Open a Pull Request from your fork's plugin-registry branch to FittyAr/Pairee:plugin-registry.\n");
    } else {
        println!("{}", t("plugin_dev_submit_sending"));
        match run_automatic_submit(&token, &commit_msg, &plugin_name).await {
            Ok(success_msg) => println!("{}", success_msg),
            Err(e) => anyhow::bail!("Automatic submission failed: {:?}", e),
        }
    }

    Ok(())
}

pub fn fetch_or_clone_registry(temp_dir: &std::path::Path) -> anyhow::Result<git2::Repository> {
    if temp_dir.join(".git").exists() {
        if let Ok(repo) = git2::Repository::open(temp_dir) {
            let fetched = {
                if let Ok(mut remote) = repo.find_remote("origin") {
                    let mut fetch_options = git2::FetchOptions::new();
                    remote.fetch(&["plugin-registry"], Some(&mut fetch_options), None).is_ok()
                } else {
                    false
                }
            };
            let mut reset_ok = false;
            let mut commit_oid = None;
            if fetched {
                if let Ok(fetch_head) = repo.find_reference("refs/remotes/origin/plugin-registry") {
                    if let Ok(commit) = fetch_head.peel_to_commit() {
                        commit_oid = Some(commit.id());
                    }
                }
            }
            if let Some(oid) = commit_oid {
                if let Ok(commit) = repo.find_commit(oid) {
                    let mut checkout_builder = git2::build::CheckoutBuilder::new();
                    checkout_builder.force();
                    let _ = repo.checkout_tree(commit.as_object(), Some(&mut checkout_builder));
                    let _ = repo.set_head("refs/heads/plugin-registry");
                    if repo.reset(commit.as_object(), git2::ResetType::Hard, None).is_ok() {
                        reset_ok = true;
                    }
                }
            }
            if reset_ok {
                return Ok(repo);
            }
        }
        // If anything fails, clean up and clone
        let _ = std::fs::remove_dir_all(temp_dir);
    }

    std::fs::create_dir_all(temp_dir)?;
    let mut builder = git2::build::RepoBuilder::new();
    builder.branch("plugin-registry");
    let repo = builder.clone("https://github.com/FittyAr/Pairee.git", temp_dir)?;
    Ok(repo)
}

pub fn package_to_registry(plugin_dir: &std::path::Path) -> anyhow::Result<String> {
    // 1. Validate the plugin
    if let Err(err_msg) = validate_for_publish(plugin_dir) {
        anyhow::bail!("Plugin validation failed: {}", err_msg);
    }

    let manifest_path = plugin_dir.join("manifest.toml");
    let content = std::fs::read_to_string(&manifest_path)?;
    let manifest = crate::plugin::loader::PluginManifest::parse(&content)?;
    let name = manifest.name.clone();

    // 2. Clone or update the registry repo in temporary directory
    let temp_dir = crate::config::paths::get_cache_dir().join("temp_registry");
    let _repo = fetch_or_clone_registry(&temp_dir)?;

    // 3. Copy plugin files to the cloned repo
    let dest_plugin_dir = temp_dir.join("registry").join(&name);
    if dest_plugin_dir.exists() {
        let _ = std::fs::remove_dir_all(&dest_plugin_dir);
    }
    std::fs::create_dir_all(&dest_plugin_dir)?;

    let mut files_hash = HashMap::new();
    for (rel_path, src_file_path) in crate::plugin::loader::get_plugin_files(plugin_dir) {
        let dest_file_path = dest_plugin_dir.join(&rel_path);
        if let Some(parent) = dest_file_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::copy(&src_file_path, &dest_file_path)?;

        let hash = crate::update::downloader::compute_sha256(&dest_file_path)?;
        files_hash.insert(rel_path, hash);
    }

    // Write sha256.sum inside the registry folder
    let mut sha_content = String::new();
    for (f, h) in &files_hash {
        sha_content.push_str(&format!("{}  {}\n", h, f));
    }
    std::fs::write(dest_plugin_dir.join("sha256.sum"), sha_content)?;

    // Copy manifest.toml to registry/<name>/manifest.toml
    std::fs::copy(&manifest_path, dest_plugin_dir.join("manifest.toml"))?;

    // 4. Update registry/index.toml
    let index_path = temp_dir.join("registry").join("index.toml");
    let mut index_data = if index_path.exists() {
        let content = std::fs::read_to_string(&index_path)?;
        toml::from_str::<crate::plugin::updater::RegistryIndex>(&content).unwrap_or_else(|_| {
            crate::plugin::updater::RegistryIndex {
                plugins: HashMap::new(),
            }
        })
    } else {
        std::fs::create_dir_all(index_path.parent().unwrap())?;
        crate::plugin::updater::RegistryIndex {
            plugins: HashMap::new(),
        }
    };

    // Construct RegistryPlugin
    let reg_plugin = crate::plugin::updater::RegistryPlugin {
        name: name.clone(),
        version: manifest.version.clone(),
        description: manifest.description.clone(),
        author: manifest.author.clone(),
        languages: manifest.languages.clone(),
        hooks: manifest.keybindings.as_ref().map(|kb| kb.values().cloned().collect()),
        min_pairee: manifest.min_pairee.clone(),
        files: files_hash,
    };

    index_data.plugins.insert(name.clone(), reg_plugin);

    // Serialize and write back
    let serialized = toml::to_string_pretty(&index_data)?;
    std::fs::write(&index_path, serialized)?;

    Ok(format!(
        "Successfully packaged '{}' v{} into the local registry branch cache.",
        name, manifest.version
    ))
}

pub fn commit_registry_changes(message: &str) -> anyhow::Result<()> {
    let temp_dir = crate::config::paths::get_cache_dir().join("temp_registry");
    let repo = git2::Repository::open(&temp_dir)?;

    let mut index = repo.index()?;
    index.add_all(["."], git2::IndexAddOption::DEFAULT, None)?;
    index.write()?;

    let oid = index.write_tree()?;
    let tree = repo.find_tree(oid)?;

    // Try to get signature from git config, fallback if none
    let signature = repo.signature().unwrap_or_else(|_| {
        git2::Signature::now("Pairee Developer", "dev@pairee.org").unwrap()
    });

    let parent_commit = repo.head()?.peel_to_commit()?;

    repo.commit(
        Some("HEAD"),
        &signature,
        &signature,
        message,
        &tree,
        &[&parent_commit],
    )?;

    Ok(())
}

pub async fn run_automatic_submit(
    token: &str,
    commit_msg: &str,
    plugin_name: &str,
) -> anyhow::Result<String> {
    let client = reqwest::Client::builder().build()?;

    // 1. Fetch user login/username
    let user_resp = client
        .get("https://api.github.com/user")
        .header("User-Agent", "Pairee-Submit-Wizard")
        .header("Authorization", format!("token {}", token))
        .header("Accept", "application/vnd.github.v3+json")
        .send()
        .await?;

    if !user_resp.status().is_success() {
        anyhow::bail!("Failed to fetch GitHub user profile: HTTP {}", user_resp.status());
    }

    let user_data: serde_json::Value = user_resp.json().await?;
    let username = user_data["login"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("GitHub username not found in response"))?
        .to_string();

    // 2. Fork repository
    let fork_url = "https://api.github.com/repos/FittyAr/Pairee/forks";
    let fork_resp = client
        .post(fork_url)
        .header("User-Agent", "Pairee-Submit-Wizard")
        .header("Authorization", format!("token {}", token))
        .header("Accept", "application/vnd.github.v3+json")
        .send()
        .await?;

    if !fork_resp.status().is_success() && fork_resp.status() != reqwest::StatusCode::ACCEPTED {
        anyhow::bail!("Failed to initiate fork: HTTP {}", fork_resp.status());
    }

    // Wait for the fork to be populated (GitHub forks are async)
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;

    // 3. Push to fork using git2
    // Place this entire block inside its own scope so all git2 non-Send types are dropped before the next .await
    {
        let temp_dir = crate::config::paths::get_cache_dir().join("temp_registry");
        let repo = git2::Repository::open(&temp_dir)?;

        let fork_git_url = format!("https://{}@github.com/{}/Pairee.git", token, username);
        let mut remote = match repo.find_remote("user_fork") {
            Ok(r) => {
                repo.remote_set_url("user_fork", &fork_git_url)?;
                r
            }
            Err(_) => repo.remote("user_fork", &fork_git_url)?,
        };

        let mut push_options = git2::PushOptions::new();
        let callbacks = git2::RemoteCallbacks::new();
        push_options.remote_callbacks(callbacks);

        remote.push(
            &["refs/heads/plugin-registry:refs/heads/plugin-registry"],
            Some(&mut push_options),
        )?;
    }

    // 4. Create Pull Request
    let pr_url = "https://api.github.com/repos/FittyAr/Pairee/pulls";
    let pr_payload = serde_json::json!({
        "title": format!("Add/Update plugin {}", plugin_name),
        "head": format!("{}:plugin-registry", username),
        "base": "plugin-registry",
        "body": commit_msg
    });

    let pr_resp = client
        .post(pr_url)
        .header("User-Agent", "Pairee-Submit-Wizard")
        .header("Authorization", format!("token {}", token))
        .header("Accept", "application/vnd.github.v3+json")
        .json(&pr_payload)
        .send()
        .await?;

    let status = pr_resp.status();
    if status.is_success() || status == reqwest::StatusCode::CREATED {
        let pr_data: serde_json::Value = pr_resp.json().await?;
        let html_url = pr_data["html_url"].as_str().unwrap_or(pr_url);
        Ok(format!("PR created successfully! View it at: {}", html_url))
    } else {
        let err_text = pr_resp.text().await.unwrap_or_default();
        anyhow::bail!(
            "Failed to create Pull Request: HTTP {}. Details: {}",
            status,
            err_text
        );
    }
}
