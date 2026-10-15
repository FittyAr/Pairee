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

/// The branch pushed to the user's fork. The `+` makes it a forced update:
/// the fork's copy is only a stage for this pull request, and a stale one
/// from an earlier submission would otherwise make every push fail as a
/// non-fast-forward.
const PUSH_REFSPEC: &str = "+refs/heads/plugin-registry:refs/heads/plugin-registry";

/// How many times git may ask for credentials. The token is one fixed
/// secret: when the first try is refused, asking again would only loop.
const MAX_CREDENTIAL_ATTEMPTS: usize = 1;

/// The reason a push of `refname` was rejected, from the server's per-ref
/// status (`None`: accepted). A push can succeed as a whole while the
/// server refuses a ref (protected branch, hook), which `push` alone does
/// not report.
fn rejected_ref(refname: &str, status: Option<&str>) -> Option<String> {
    status.map(|why| format!("{refname}: {why}"))
}

/// Pushes the registry branch of the clone in `temp_dir` to the user's
/// fork. The token is handed to git through the credentials callback, so
/// it is never stored in the clone's `.git/config` remote URL. A refused
/// credential or a ref the server rejects is an error.
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
    let rejected = std::cell::RefCell::new(Vec::new());
    let mut callbacks = git2::RemoteCallbacks::new();
    let mut attempts = 0;
    callbacks.credentials(|_, _, allowed| {
        attempts += 1;
        if attempts > MAX_CREDENTIAL_ATTEMPTS || !allowed.is_user_pass_plaintext() {
            return Err(git2::Error::from_str(
                "GitHub refused the token (check that it is valid and can push to your fork)",
            ));
        }
        git2::Cred::userpass_plaintext(username, token)
    });
    callbacks.push_update_reference(|refname, status| {
        if let Some(why) = rejected_ref(refname, status) {
            rejected.borrow_mut().push(why);
        }
        Ok(())
    });
    let mut push_options = git2::PushOptions::new();
    push_options.remote_callbacks(callbacks);
    remote.push(&[PUSH_REFSPEC], Some(&mut push_options))?;
    let rejected = rejected.take();
    if !rejected.is_empty() {
        anyhow::bail!("GitHub rejected the push: {}", rejected.join("; "));
    }
    Ok(())
}

/// Calls `check` up to `attempts` times, `interval` apart, until it
/// yields a value.
async fn poll<T, Fut: std::future::Future<Output = Option<T>>>(
    attempts: u32,
    interval: std::time::Duration,
    mut check: impl FnMut() -> Fut,
) -> Option<T> {
    for attempt in 0..attempts {
        if let Some(found) = check().await {
            return Some(found);
        }
        if attempt + 1 < attempts {
            tokio::time::sleep(interval).await;
        }
    }
    None
}

/// Waits (up to a minute) until GitHub has finished creating the fork;
/// forking is asynchronous, so the repository answers 404 for a while.
async fn wait_for_fork(
    client: &reqwest::Client,
    token: &str,
    username: &str,
) -> anyhow::Result<()> {
    let url = format!("https://api.github.com/repos/{username}/Pairee");
    let ready = poll(30, std::time::Duration::from_secs(2), || async {
        let response = github(client.get(&url), token).send().await.ok()?;
        response.status().is_success().then_some(())
    })
    .await;
    ready.ok_or_else(|| anyhow::anyhow!(t("plugin_dev_err_fork_timeout")))
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

    // GitHub creates forks asynchronously: wait until it exists.
    wait_for_fork(&client, token, &username).await?;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_push_is_forced_and_credentials_are_asked_once() {
        assert!(PUSH_REFSPEC.starts_with('+'));
        assert_eq!(MAX_CREDENTIAL_ATTEMPTS, 1);
    }

    #[test]
    fn a_ref_the_server_refuses_is_reported() {
        assert_eq!(rejected_ref("refs/heads/x", None), None);
        let why = rejected_ref("refs/heads/x", Some("protected branch")).unwrap();
        assert!(why.contains("refs/heads/x") && why.contains("protected branch"));
    }

    #[tokio::test]
    async fn polling_stops_at_the_first_answer_and_gives_up() {
        let calls = std::cell::Cell::new(0);
        let found = poll(5, std::time::Duration::ZERO, || {
            calls.set(calls.get() + 1);
            let n = calls.get();
            async move { (n == 3).then_some(n) }
        })
        .await;
        assert_eq!(found, Some(3));
        assert_eq!(calls.get(), 3);
        let never: Option<()> = poll(2, std::time::Duration::ZERO, || async { None }).await;
        assert_eq!(never, None);
    }
}
