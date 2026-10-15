//! minisign signatures of the plugin registry (`index.toml`,
//! `blocklist.toml` and every plugin `manifest.toml`, whose `[files]`
//! table pins the SHA-256 of each plugin file).
//!
//! Each signed file `X` may have `X.minisig` next to it. The key is pinned
//! in the binary: the Pairee release key, the same one the self-updater
//! checks releases with, verified with the same code
//! ([`crate::update::signature::verify_with_key`]); `registry_public_key`
//! under `[security]` replaces it for a fork. Nothing is learned from the
//! network: the first signed file an installation sees is checked against
//! that key like every later one. The trusted comment must name the file by
//! its path inside `registry/` (`file:index.toml`,
//! `file:plugins/a/alice/demo/manifest.toml`), so a signature cannot be
//! replayed for another file.
//!
//! **Freshness.** The trusted comment of `index.toml` and `blocklist.toml`
//! (the files that say which versions exist and which are blocked) must
//! also carry `serial:<N>` and `timestamp:<unix seconds>` (written by
//! `cargo xtask sign-registry`). The highest serial accepted per file is
//! kept in [`REGISTRY_STATE_FILE`] in the config folder (not in a plugin
//! data folder, which plugins can write); an older serial is refused
//! (rollback), and so is a signature older than `registry_max_age_days`
//! (a frozen registry or mirror). Plugin manifests need no freshness: the
//! signed index names the version each must carry.
//!
//! **Missing signatures.**
//! * A signature that is present but does not verify is always an error.
//! * With `registry_require_signature = true` (the default), a missing
//!   signature is refused once [`REGISTRY_SIGNED`] declares the official
//!   registry signed. Until then (the registry is not signed yet) it is
//!   refused only after this installation has verified a signed registry
//!   file with the configured key (stripping the signatures cannot
//!   downgrade it), and otherwise accepted with an "unsigned" warning.
//! * `false` always only warns.

use crate::config::localization::t_fill;
use crate::config::settings::Settings;
use crate::config::toml_store::StateFile;
use crate::update::signature::comment_field;
use anyhow::{Result, anyhow, bail};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Whether the official registry publishes a signature for every file.
/// Flip it to `true` once the `plugin-registry` branch is signed (see
/// `docs/technical/plugin-registry-spec.md`): from then on
/// `registry_require_signature` refuses unsigned files from the very first
/// fetch, with no grace period.
pub const REGISTRY_SIGNED: bool = false;

/// The registry files whose signature must carry `serial:` and `timestamp:`.
pub const FRESHNESS_TRACKED: [&str; 2] = ["index.toml", "blocklist.toml"];

/// File in the config folder recording what this installation accepted.
pub const REGISTRY_STATE_FILE: &str = "registry_state.toml";

const SECONDS_PER_DAY: u64 = 86_400;

/// How a registry file was authenticated.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SignatureStatus {
    Verified,
    /// No `.minisig` was published for the file.
    Unsigned,
}

/// What this installation accepted from the registry, for one key.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct RegistryState {
    /// The key the entries below were verified with (empty: none yet).
    pub key: String,
    /// Per freshness-tracked file: the newest signature accepted.
    pub files: BTreeMap<String, Freshness>,
}

impl StateFile for RegistryState {
    const FILE_NAME: &'static str = REGISTRY_STATE_FILE;
}

/// The `serial:` and `timestamp:` of a signed registry file.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Freshness {
    pub serial: u64,
    /// Unix seconds when it was signed.
    pub timestamp: u64,
}

impl Freshness {
    /// Reads them from a verified trusted comment.
    fn parse(trusted_comment: &str) -> Option<Self> {
        let number = |name| comment_field(trusted_comment, name)?.parse().ok();
        Some(Self {
            serial: number("serial")?,
            timestamp: number("timestamp")?,
        })
    }
}

impl RegistryState {
    /// The state recorded for `key` (another key starts from nothing).
    fn for_key(self, key: &str) -> Self {
        if self.key == key {
            self
        } else {
            Self {
                key: key.to_string(),
                files: BTreeMap::new(),
            }
        }
    }

    /// Records that `name` was accepted with `fresh` (never lowering it).
    fn accept(mut self, name: &str, fresh: Option<Freshness>) -> Self {
        if let Some(fresh) = fresh {
            let entry = self.files.entry(name.to_string()).or_default();
            if fresh.serial >= entry.serial {
                *entry = fresh;
            }
        }
        self
    }
}

/// The key and policy registry files are checked with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegistryTrust {
    /// minisign public key (base64).
    pub public_key: String,
    /// Refuse files without a signature (see the module docs).
    pub require_signature: bool,
    /// The registry is known to be signed ([`REGISTRY_SIGNED`]): an unsigned
    /// file is refused even before a signed one was seen.
    pub registry_signed: bool,
    /// Oldest accepted index / blocklist signature, in days (`0`: any).
    pub max_age_days: u32,
    /// Where accepted serials are recorded (`None`: nowhere, so no
    /// rollback check across calls).
    pub state_path: Option<PathBuf>,
}

impl Default for RegistryTrust {
    fn default() -> Self {
        Self {
            public_key: crate::update::signature::RELEASE_PUBLIC_KEY.to_string(),
            require_signature: true,
            registry_signed: REGISTRY_SIGNED,
            max_age_days: crate::config::settings::types::DEFAULT_REGISTRY_MAX_AGE_DAYS,
            state_path: None,
        }
    }
}

fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

impl RegistryTrust {
    /// The registry trust configured in `settings`.
    pub fn from_settings(settings: &Settings) -> Self {
        let security = &settings.security;
        let key = security.registry_public_key.trim();
        Self {
            public_key: if key.is_empty() {
                Self::default().public_key
            } else {
                key.to_string()
            },
            require_signature: security.registry_require_signature,
            max_age_days: security.registry_max_age_days,
            state_path: Some(RegistryState::path()),
            ..Self::default()
        }
    }

    /// What was recorded for the configured key.
    fn state(&self) -> RegistryState {
        self.state_path
            .as_deref()
            .and_then(RegistryState::read_from)
            .unwrap_or_default()
            .for_key(&self.public_key)
    }

    /// Whether every registry file must carry a valid signature: signatures
    /// are required and the registry is declared signed or this
    /// installation has verified a signed file before. A file that cannot
    /// be fetched is then an error, not an empty list.
    pub fn expects_signed(&self) -> bool {
        self.require_signature && (self.registry_signed || self.seen_signed())
    }

    /// Whether a file signed with this key has been verified before.
    fn seen_signed(&self) -> bool {
        self.state_path
            .as_deref()
            .and_then(RegistryState::read_from)
            .is_some_and(|state| state.key == self.public_key)
    }

    /// Records the accepted file (and that the registry is signed).
    fn record(&self, path: &Path, name: &str, fresh: Option<Freshness>) {
        let key = &self.public_key;
        let saved = RegistryState::merge_save_to(path, |disk| {
            disk.unwrap_or_default().for_key(key).accept(name, fresh)
        });
        if let Err(e) = saved {
            tracing::warn!("Cannot record the plugin registry state: {e:#}");
        }
    }

    /// Checks `data` (the registry file `signed_name`) against its
    /// `.minisig` text, if one was published.
    pub fn verify(
        &self,
        data: &[u8],
        minisig: Option<&str>,
        signed_name: &str,
    ) -> Result<SignatureStatus> {
        self.verify_at(data, minisig, signed_name, now_unix())
    }

    /// [`Self::verify`] at the time `now` (unix seconds).
    pub fn verify_at(
        &self,
        data: &[u8],
        minisig: Option<&str>,
        signed_name: &str,
        now: u64,
    ) -> Result<SignatureStatus> {
        let Some(sig) = minisig else {
            return self.unsigned(signed_name);
        };
        let comment =
            crate::update::signature::verify_with_key(&self.public_key, data, sig, signed_name)
                .map_err(|e| anyhow!("registry signature of {signed_name} is invalid: {e:#}"))?;
        let fresh = if FRESHNESS_TRACKED.contains(&signed_name) {
            Some(self.check_freshness(&comment, signed_name, now)?)
        } else {
            None
        };
        if let Some(path) = &self.state_path
            && (fresh.is_some() || !self.seen_signed())
        {
            self.record(path, signed_name, fresh);
        }
        Ok(SignatureStatus::Verified)
    }

    /// A file published without a signature.
    fn unsigned(&self, signed_name: &str) -> Result<SignatureStatus> {
        if self.expects_signed() {
            bail!(t_fill("plugin_registry_err_unsigned", &[signed_name]));
        }
        tracing::warn!("Plugin registry file {signed_name} is not signed");
        Ok(SignatureStatus::Unsigned)
    }

    /// The serial and timestamp of a verified `comment`: not older than
    /// what was accepted before, nor than `max_age_days`.
    fn check_freshness(&self, comment: &str, signed_name: &str, now: u64) -> Result<Freshness> {
        let fresh = Freshness::parse(comment)
            .ok_or_else(|| anyhow!(t_fill("plugin_registry_err_no_freshness", &[signed_name])))?;
        if let Some(seen) = self.state().files.get(signed_name)
            && fresh.serial < seen.serial
        {
            bail!(t_fill(
                "plugin_registry_err_rollback",
                &[
                    signed_name,
                    &fresh.serial.to_string(),
                    &seen.serial.to_string()
                ],
            ));
        }
        let age_days = now.saturating_sub(fresh.timestamp) / SECONDS_PER_DAY;
        if self.max_age_days > 0 && age_days > u64::from(self.max_age_days) {
            bail!(t_fill(
                "plugin_registry_err_stale",
                &[
                    signed_name,
                    &age_days.to_string(),
                    &self.max_age_days.to_string()
                ],
            ));
        }
        Ok(fresh)
    }
}

#[cfg(test)]
#[path = "trust_tests.rs"]
mod tests;
