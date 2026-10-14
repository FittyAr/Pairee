use super::progress_status;
use crate::app::state::DevProgress;
use crate::config::localization::t;
use tokio::sync::mpsc::UnboundedSender;

pub fn commit_registry_changes_with_progress(
    message: &str,
    progress: Option<UnboundedSender<DevProgress>>,
) -> anyhow::Result<()> {
    progress_status(&progress, t("plugin_dev_progress_staging"));
    let temp_dir = crate::plugin::developer_tool::temp_registry_dir();
    let repo = git2::Repository::open(&temp_dir)?;

    let mut index = repo.index()?;
    index.add_all(["."], git2::IndexAddOption::DEFAULT, None)?;
    index.write()?;

    let oid = index.write_tree()?;
    let tree = repo.find_tree(oid)?;

    // Try to get signature from git config, fallback if none
    let signature = match repo.signature() {
        Ok(s) => s,
        Err(_) => git2::Signature::now("Pairee Developer", "dev@pairee.org").map_err(|e| {
            anyhow::anyhow!(
                crate::config::localization::t("plugin_dev_signature_failed")
                    .replace("{}", &e.to_string())
            )
        })?,
    };

    let parent_commit = repo.head()?.peel_to_commit()?;

    progress_status(&progress, t("plugin_dev_progress_committing"));
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

/// Longest a single GitHub API request may take.
const GITHUB_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(60);

/// A GitHub API request authenticated with `token`.
fn github(request: reqwest::RequestBuilder, token: &str) -> reqwest::RequestBuilder {
    request
        .header("User-Agent", "Pairee-Submit-Wizard")
        .header("Authorization", format!("token {token}"))
        .header("Accept", "application/vnd.github.v3+json")
}

/// Pushes the registry branch of the clone in `temp_dir` to the user's
/// fork. The token is handed to git through the credentials callback, so
/// it is never stored in the clone's `.git/config` remote URL.
fn push_to_fork(temp_dir: &std::path::Path, username: &str, token: &str) -> anyhow::Result<()> {
    let repo = git2::Repository::open(temp_dir)?;
    let fork_git_url = format!("https://github.com/{username}/Pairee.git");
    let mut remote = match repo.find_remote("user_fork") {
        Ok(_) => {
            repo.remote_set_url("user_fork", &fork_git_url)?;
            repo.find_remote("user_fork")?
        }
        Err(_) => repo.remote("user_fork", &fork_git_url)?,
    };
    let mut callbacks = git2::RemoteCallbacks::new();
    callbacks.credentials(|_, _, _| git2::Cred::userpass_plaintext(username, token));
    let mut push_options = git2::PushOptions::new();
    push_options.remote_callbacks(callbacks);
    remote.push(
        &["refs/heads/plugin-registry:refs/heads/plugin-registry"],
        Some(&mut push_options),
    )?;
    Ok(())
}

pub async fn run_automatic_submit(
    token: &str,
    commit_msg: &str,
    plugin_name: &str,
) -> anyhow::Result<String> {
    let client = crate::update::http::client(GITHUB_TIMEOUT)?;

    // 1. Fetch user login/username
    let user_resp = github(client.get("https://api.github.com/user"), token)
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
    let fork_resp = github(client.post(fork_url), token).send().await?;

    if !fork_resp.status().is_success() && fork_resp.status() != reqwest::StatusCode::ACCEPTED {
        anyhow::bail!(t("plugin_dev_err_fork").replace("{}", &fork_resp.status().to_string()));
    }

    // Wait for the fork to be populated (GitHub forks are async)
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;

    // 3. Push to fork using git2 (a sync helper: its non-Send git2 types
    // never live across an .await)
    push_to_fork(
        &crate::plugin::developer_tool::temp_registry_dir(),
        &username,
        token,
    )?;

    // 4. Create Pull Request
    let pr_url = "https://api.github.com/repos/FittyAr/Pairee/pulls";
    let pr_payload = serde_json::json!({
        "title": format!("Add/Update plugin {}", plugin_name),
        "head": format!("{}:plugin-registry", username),
        "base": "plugin-registry",
        "body": commit_msg
    });

    let pr_resp = github(client.post(pr_url), token)
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
