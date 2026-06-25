use anyhow::{Context as _, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const GITHUB_API_LATEST: &str =
    "https://api.github.com/repos/FittyAr/Pairee/releases/latest";
const CACHE_TTL_SECS: u64 = 3600; // 1 hour

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

fn save_cache(release: &GhRelease) {
    let cached = CachedRelease {
        fetched_at: now_secs(),
        tag_name: release.tag_name.clone(),
        html_url: release.html_url.clone(),
        body: release.body.clone().unwrap_or_default(),
        assets: release
            .assets
            .iter()
            .map(|a| CachedAsset {
                name: a.name.clone(),
                url: a.browser_download_url.clone(),
                size: a.size,
            })
            .collect(),
    };
    if let Ok(json) = serde_json::to_string_pretty(&cached) {
        let _ = std::fs::write(cache_path(), json);
    }
}

// ─── Version comparison (simple semver: MAJOR.MINOR.PATCH) ──────────────────

fn parse_version(v: &str) -> Option<(u32, u32, u32)> {
    let v = v.trim_start_matches('v');
    // Accept "0.5.0" or "0.5.0-beta" (just compare numeric parts)
    let parts: Vec<&str> = v.splitn(2, '-').next()?.split('.').collect();
    if parts.len() < 3 {
        if parts.len() == 2 {
            return Some((parts[0].parse().ok()?, parts[1].parse().ok()?, 0));
        }
        return None;
    }
    Some((
        parts[0].parse().ok()?,
        parts[1].parse().ok()?,
        parts[2].parse().ok()?,
    ))
}

fn is_newer(latest_tag: &str, current: &str) -> bool {
    match (parse_version(latest_tag), parse_version(current)) {
        (Some(l), Some(c)) => l > c,
        _ => false,
    }
}

// ─── Public API ─────────────────────────────────────────────────────────────

pub struct UpdateChecker;

impl UpdateChecker {
    /// Performs a version check (respects 1-hour cache).
    /// Returns `Some(UpdateInfo)` when a newer version is available, `None` otherwise.
    pub async fn check() -> Result<Option<UpdateInfo>> {
        let current = env!("CARGO_PKG_VERSION");

        // Try cache first
        if let Some(cached) = load_cache() {
            if !is_newer(&cached.tag_name, current) {
                return Ok(None);
            }
            let notes = extract_notes(&cached.body);
            return Ok(Some(UpdateInfo {
                tag: cached.tag_name.clone(),
                version: cached.tag_name.trim_start_matches('v').to_string(),
                release_notes: notes,
                html_url: cached.html_url,
                assets: cached
                    .assets
                    .iter()
                    .map(|a| ReleaseAsset {
                        name: a.name.clone(),
                        browser_download_url: a.url.clone(),
                        size: a.size,
                    })
                    .collect(),
            }));
        }

        // Fetch from GitHub
        let client = build_client()?;
        let response = client
            .get(GITHUB_API_LATEST)
            .header("User-Agent", format!("pairee/{}", current))
            .header("Accept", "application/vnd.github+json")
            .send()
            .await
            .context("failed to reach GitHub API")?;

        if !response.status().is_success() {
            anyhow::bail!("GitHub API returned status {}", response.status());
        }

        let release: GhRelease = response
            .json()
            .await
            .context("failed to parse GitHub release JSON")?;

        save_cache(&release);

        if !is_newer(&release.tag_name, current) {
            return Ok(None);
        }

        let body_text = release.body.clone().unwrap_or_default();
        let notes = extract_notes(&body_text);

        Ok(Some(UpdateInfo {
            tag: release.tag_name.clone(),
            version: release.tag_name.trim_start_matches('v').to_string(),
            release_notes: notes,
            html_url: release.html_url.clone(),
            assets: release
                .assets
                .iter()
                .map(|a| ReleaseAsset {
                    name: a.name.clone(),
                    browser_download_url: a.browser_download_url.clone(),
                    size: a.size,
                })
                .collect(),
        }))
    }

    /// Spawn a background task that checks for updates and sends the result
    /// over the provided oneshot channel.
    pub fn check_in_background(
        tx: tokio::sync::oneshot::Sender<Option<UpdateInfo>>,
    ) {
        tokio::spawn(async move {
            match Self::check().await {
                Ok(info) => {
                    let _ = tx.send(info);
                }
                Err(e) => {
                    log::debug!("Update check failed: {}", e);
                    let _ = tx.send(None);
                }
            }
        });
    }
}

/// Extract a short summary from the release body markdown.
fn extract_notes(body: &str) -> String {
    // Take up to the first 5 non-empty lines (ignoring markdown headings)
    let lines: Vec<&str> = body
        .lines()
        .filter(|l| !l.trim().is_empty())
        .take(6)
        .collect();
    lines.join("\n")
}

fn build_client() -> Result<reqwest::Client> {
    #[cfg(not(target_os = "windows"))]
    {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .build()
            .context("failed to build HTTP client")?;
        Ok(client)
    }
    #[cfg(target_os = "windows")]
    {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .build()
            .context("failed to build HTTP client")?;
        Ok(client)
    }
}

// ─── Unit tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_version() {
        assert_eq!(parse_version("v0.5.0"), Some((0, 5, 0)));
        assert_eq!(parse_version("0.4.1"), Some((0, 4, 1)));
        assert_eq!(parse_version("1.0.0-beta"), Some((1, 0, 0)));
        assert_eq!(parse_version("v2.3"), Some((2, 3, 0)));
        assert_eq!(parse_version("invalid"), None);
    }

    #[test]
    fn test_is_newer() {
        assert!(is_newer("v0.5.0", "0.4.1"));
        assert!(!is_newer("v0.4.1", "0.4.1"));
        assert!(!is_newer("v0.3.0", "0.4.1"));
        assert!(is_newer("v1.0.0", "0.9.9"));
    }

    #[test]
    fn test_extract_notes() {
        let body = "## What's New\n\nAdded feature A\nFixed bug B\n\n## Breaking\n\nNone";
        let notes = extract_notes(body);
        assert!(!notes.is_empty());
    }
}
