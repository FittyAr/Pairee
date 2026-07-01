use super::package::{package_to_registry, validate_for_publish};
use crate::config::localization::t;

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
    println!("{}", t("plugin_enter_commit_desc"));
    let mut commit_msg = String::new();
    std::io::stdin().read_line(&mut commit_msg)?;
    let commit_msg = commit_msg.trim().to_string();
    if commit_msg.is_empty() {
        anyhow::bail!("Commit message cannot be empty.");
    }

    // 3. Commit locally
    commit_registry_changes(&commit_msg)?;
    println!("{}", t("plugin_dev_local_staged"));

    // 4. Prompt for token (optional)
    println!("{}", t("plugin_dev_submit_prompt")); // "Enter GitHub Token: "
    let mut token = String::new();
    std::io::stdin().read_line(&mut token)?;
    let token = token.trim().to_string();

    if token.is_empty() {
        let temp_dir = crate::config::paths::get_cache_dir().join("temp_registry");
        let msg = t("plugin_dev_no_token_inst").replace("{}", &temp_dir.display().to_string());
        println!("\n{}", msg);
    } else {
        println!("{}", t("plugin_dev_submit_sending"));
        match run_automatic_submit(&token, &commit_msg, &plugin_name).await {
            Ok(success_msg) => println!("{}", success_msg),
            Err(e) => anyhow::bail!("Automatic submission failed: {:?}", e),
        }
    }

    Ok(())
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
    let signature = repo
        .signature()
        .unwrap_or_else(|_| git2::Signature::now("Pairee Developer", "dev@pairee.org").unwrap());

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
        anyhow::bail!(
            t("plugin_dev_err_user_profile").replace("{}", &user_resp.status().to_string())
        );
    }

    let user_data: serde_json::Value = user_resp.json().await?;
    let username = user_data["login"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!(t("plugin_dev_err_username_not_found").to_string()))?
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
        anyhow::bail!(t("plugin_dev_err_fork").replace("{}", &fork_resp.status().to_string()));
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
        Ok(t("plugin_dev_pr_created").replace("{}", html_url))
    } else {
        let err_text = pr_resp.text().await.unwrap_or_default();
        let err_pattern = t("plugin_dev_err_pr");
        let err_msg = err_pattern
            .replacen("{}", &status.to_string(), 1)
            .replacen("{}", &err_text, 1);
        anyhow::bail!(err_msg);
    }
}
