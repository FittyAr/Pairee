use anyhow::{Context as _, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const GITHUB_API_LATEST: &str = "https://api.github.com/repos/FittyAr/Pairee/releases/latest";
const CACHE_TTL_SECS: u64 = 3600; // 1 hour
/// Whole-request timeout of the release check.
const CHECK_TIMEOUT: Duration = Duration::from_secs(15);
/// Largest GitHub "latest release" API response accepted.
const MAX_API_BYTES: usize = 4 * 1024 * 1024;

/// Parsed information about an available update.
#[derive(Debug, Clone)]
pub struct UpdateInfo {
    /// Tag name from GitHub, e.g. "v0.5.0"
    pub tag: String,
    /// Parsed version string without leading "v", e.g. "0.5.0"
    pub version: String,
    /// Short description / first paragraph of the release body
    pub release_notes: String,
    /// HTML URL for the release page
    pub html_url: String,
    /// All assets in the release
    pub assets: Vec<ReleaseAsset>,
}

#[derive(Debug, Clone)]
pub struct ReleaseAsset {
    pub name: String,
    pub browser_download_url: String,
    pub size: u64,
}

// ─── GitHub JSON shapes ─────────────────────────────────────────────────────

#[derive(Deserialize)]
struct GhRelease {
    tag_name: String,
    html_url: String,
    body: Option<String>,
    assets: Vec<GhAsset>,
}

#[derive(Deserialize)]
struct GhAsset {
    name: String,
    browser_download_url: String,
    size: u64,
}

// ─── Cache on disk ───────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize)]
struct CachedRelease {
    /// Unix timestamp when the cache was written
    fetched_at: u64,
    tag_name: String,
    html_url: String,
    body: String,
    assets: Vec<CachedAsset>,
}

#[derive(Serialize, Deserialize)]
struct CachedAsset {
    name: String,
    url: String,
    size: u64,
}

fn cache_path() -> PathBuf {
    crate::config::paths::get_config_dir().join("update_cache.json")
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_secs()
}

fn load_cache() -> Option<CachedRelease> {
    let path = cache_path();
    let data = std::fs::read_to_string(&path).ok()?;
    let cached: CachedRelease = serde_json::from_str(&data).ok()?;
    if now_secs().saturating_sub(cached.fetched_at) < CACHE_TTL_SECS {
        Some(cached)
    } else {
        None
    }
}

impl From<GhRelease> for CachedRelease {
    fn from(release: GhRelease) -> Self {
        Self {
            fetched_at: now_secs(),
            tag_name: release.tag_name,
            html_url: release.html_url,
            body: release.body.unwrap_or_default(),
            assets: release
                .assets
                .into_iter()
                .map(|a| CachedAsset {
                    name: a.name,
                    url: a.browser_download_url,
                    size: a.size,
                })
                .collect(),
        }
    }
}

fn save_cache(cached: &CachedRelease) {
    if let Ok(json) = serde_json::to_string_pretty(cached)
        && let Err(e) = std::fs::write(cache_path(), json)
    {
        log::warn!("Failed to write update cache: {}", e);
    }
}

/// The update `release` offers over `current`, if it is newer.
fn newer_release(release: CachedRelease, current: &str) -> Option<UpdateInfo> {
    if !super::version::is_newer(&release.tag_name, current) {
        return None;
    }
    Some(UpdateInfo {
        version: release.tag_name.trim_start_matches('v').to_string(),
        tag: release.tag_name,
        release_notes: extract_notes(&release.body),
        html_url: release.html_url,
        assets: release
            .assets
            .into_iter()
            .map(|a| ReleaseAsset {
                name: a.name,
                browser_download_url: a.url,
                size: a.size,
            })
            .collect(),
    })
}

// ─── Public API ─────────────────────────────────────────────────────────────

pub struct UpdateChecker;

impl UpdateChecker {
    /// Performs a version check (respects 1-hour cache).
    /// Returns `Some(UpdateInfo)` when a newer version is available, `None` otherwise.
    pub async fn check() -> Result<Option<UpdateInfo>> {
        let current = env!("CARGO_PKG_VERSION");
        tokio::task::spawn_blocking(super::installer::remove_stale_files);

        if let Some(cached) = load_cache() {
            return Ok(newer_release(cached, current));
        }

        let client = super::http::client(CHECK_TIMEOUT)?;
        let response = client
            .get(GITHUB_API_LATEST)
            .header("Accept", "application/vnd.github+json")
            .send()
            .await
            .context("failed to reach GitHub API")?;

        if !response.status().is_success() {
            anyhow::bail!("GitHub API returned status {}", response.status());
        }

        let body = super::http::read_limited(response, MAX_API_BYTES, |_, _| {})
            .await
            .context("failed to read the GitHub API response")?;
        let release: GhRelease =
            serde_json::from_slice(&body).context("failed to parse GitHub release JSON")?;
        let cached = CachedRelease::from(release);
        save_cache(&cached);
        Ok(newer_release(cached, current))
    }

    /// Forgets the cached release, so the next check asks GitHub.
    pub fn forget_cache() {
        let _ = std::fs::remove_file(cache_path());
    }

    /// Spawn a background task that checks for updates and sends the result
    /// over the provided oneshot channel.
    pub fn check_in_background(tx: tokio::sync::oneshot::Sender<Option<UpdateInfo>>) {
        tokio::spawn(async move { deliver(tx, Self::check().await) });
    }
}

/// Sends the outcome of a check. A failed check drops `tx` without sending,
/// so the receiver tells "check failed" apart from "no update".
fn deliver(
    tx: tokio::sync::oneshot::Sender<Option<UpdateInfo>>,
    result: Result<Option<UpdateInfo>>,
) {
    match result {
        Ok(info) => {
            let _ = tx.send(info);
        }
        Err(e) => log::debug!("Update check failed: {}", e),
    }
}

/// Extract a short summary from the release body markdown.
fn extract_notes(body: &str) -> String {
    // Take up to the first 5 non-empty lines (ignoring markdown headings)
    let lines: Vec<&str> = body
        .lines()
        .filter(|l| !l.trim().is_empty() && !l.trim_start().starts_with('#'))
        .take(5)
        .collect();
    lines.join("\n")
}

// ─── Unit tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_failed_check_is_not_reported_as_no_update() {
        use tokio::sync::oneshot::error::TryRecvError;
        let (tx, mut rx) = tokio::sync::oneshot::channel();
        deliver(tx, Err(anyhow::anyhow!("offline")));
        assert_eq!(rx.try_recv().unwrap_err(), TryRecvError::Closed);

        let (tx, mut rx) = tokio::sync::oneshot::channel();
        deliver(tx, Ok(None));
        assert!(rx.try_recv().unwrap().is_none());
    }

    #[test]
    fn only_newer_releases_are_offered() {
        let release = |tag: &str| CachedRelease {
            fetched_at: 0,
            tag_name: tag.into(),
            html_url: String::new(),
            body: String::new(),
            assets: vec![CachedAsset {
                name: "a.zip".into(),
                url: "u".into(),
                size: 1,
            }],
        };
        assert!(newer_release(release("v0.4.0"), "0.4.0").is_none());
        let info = newer_release(release("v0.5.0"), "0.4.0").unwrap();
        assert_eq!(
            (info.tag.as_str(), info.version.as_str()),
            ("v0.5.0", "0.5.0")
        );
        assert_eq!(info.assets[0].browser_download_url, "u");
    }

    #[test]
    fn test_extract_notes() {
        let body = "## What's New\n\nAdded feature A\nFixed bug B\n\n## Breaking\n\nNone";
        let notes = extract_notes(body);
        assert_eq!(
            notes,
            "Added feature A
Fixed bug B
None"
        );
    }

    #[test]
    fn extract_notes_keeps_at_most_five_lines() {
        let body = "# T
1
2
3
4
5
6
7";
        assert_eq!(
            extract_notes(body),
            "1
2
3
4
5"
        );
    }
}
