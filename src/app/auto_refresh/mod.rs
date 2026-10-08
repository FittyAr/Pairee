//! Panel auto-refresh: keeps one [`DirMonitor`] per folder shown by a
//! visible local tab (and, with `auto_refresh_ssh`, per SFTP folder, polled
//! over the connection), re-arming them as tabs move or close, and hands
//! the coalesced changes to [`AppState::poll_auto_refresh`] (see `apply.rs`).
//!
//! [`AppState::poll_auto_refresh`]: crate::app::state::AppState::poll_auto_refresh

mod apply;
#[cfg(test)]
mod tests;

use crate::config::settings::Settings;
use crate::fs::ssh::SharedSshClient;
use crate::fs::watch::{
    ChangeSink, CoalesceTiming, Coalescer, DirChange, DirMonitor, MonitorPlan, WatchOrigin,
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::{Duration, Instant};

/// A folder that should be monitored.
#[derive(Debug, Clone)]
pub struct WatchTarget {
    pub dir: PathBuf,
    /// The SFTP connection of a remote folder (`None`: local disk).
    pub remote: Option<SharedSshClient>,
}

impl WatchTarget {
    pub fn origin(&self) -> WatchOrigin {
        origin_of(self.remote.as_ref())
    }
}

/// The watch origin of a folder on `remote` (or on the local disk).
pub fn origin_of(remote: Option<&SharedSshClient>) -> WatchOrigin {
    remote.map_or(WatchOrigin::Local, |c| WatchOrigin::Remote(c.id()))
}

/// Monitors are told apart by filesystem and folder.
type WatchKey = (WatchOrigin, PathBuf);

fn seconds(secs: u32) -> Duration {
    Duration::from_secs(u64::from(secs.max(1)))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Config {
    enabled: bool,
    poll_interval: Duration,
    /// SFTP folders are polled too.
    ssh: bool,
    ssh_interval: Duration,
}

impl Config {
    fn from_settings(settings: &Settings) -> Self {
        Self {
            enabled: settings.auto_refresh,
            poll_interval: seconds(settings.auto_refresh_poll_secs),
            ssh: settings.auto_refresh_ssh,
            ssh_interval: seconds(settings.auto_refresh_ssh_poll_secs),
        }
    }
}

pub struct AutoRefresh {
    config: Config,
    monitors: HashMap<WatchKey, (MonitorPlan, DirMonitor)>,
    coalescer: Coalescer,
    tx: Sender<DirChange>,
    rx: Receiver<DirChange>,
}

impl Default for AutoRefresh {
    fn default() -> Self {
        let (tx, rx) = channel();
        Self {
            config: Config::from_settings(&Settings::default()),
            monitors: HashMap::new(),
            coalescer: Coalescer::new(CoalesceTiming::default()),
            tx,
            rx,
        }
    }
}

impl AutoRefresh {
    /// Applies the `auto_refresh*` settings; monitors are re-armed on the
    /// next [`Self::sync`].
    pub fn configure(&mut self, settings: &Settings) {
        let config = Config::from_settings(settings);
        if config != self.config {
            self.config = config;
            self.monitors.clear();
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Monitors exactly `targets` (none when disabled; SFTP folders only
    /// with `auto_refresh_ssh`): monitors of folders no longer shown stop,
    /// new folders start being monitored.
    pub fn sync(&mut self, targets: Vec<WatchTarget>) {
        let config = self.config;
        let wanted: HashMap<WatchKey, (MonitorPlan, WatchTarget)> = targets
            .into_iter()
            .filter(|t| config.enabled && (t.remote.is_none() || config.ssh))
            .map(|t| ((t.origin(), t.dir.clone()), (self.plan(&t), t)))
            .collect();
        self.monitors
            .retain(|key, (plan, _)| wanted.get(key).is_some_and(|(p, _)| p == plan));
        for (key, (plan, target)) in wanted {
            if self.monitors.contains_key(&key) {
                continue;
            }
            let sink = ChangeSink::new(self.tx.clone(), crate::app::jobs::wake_event_loop)
                .with_origin(key.0);
            let monitor = match target.remote {
                Some(client) => DirMonitor::start_polling(
                    target.dir,
                    Arc::new(client),
                    plan.poll_interval,
                    sink,
                ),
                None => DirMonitor::start(target.dir, plan, sink),
            };
            self.monitors.insert(key, (plan, monitor));
        }
        let monitors = &self.monitors;
        self.coalescer
            .retain(|origin, dir| monitors.contains_key(&(origin, dir.to_path_buf())));
    }

    /// Remote folders are polled on their own (longer) interval.
    fn plan(&self, target: &WatchTarget) -> MonitorPlan {
        let remote = target.remote.is_some();
        MonitorPlan {
            prefer_poll: remote,
            poll_interval: if remote {
                self.config.ssh_interval
            } else {
                self.config.poll_interval
            },
        }
    }

    /// Folders currently monitored, sorted.
    pub fn monitored(&self) -> Vec<&Path> {
        let mut dirs: Vec<&Path> = self.monitors.keys().map(|(_, d)| d.as_path()).collect();
        dirs.sort();
        dirs
    }

    /// Collects the reported changes and returns the folders due at `now`.
    pub fn take_due(&mut self, now: Instant) -> Vec<DirChange> {
        while let Ok(change) = self.rx.try_recv() {
            if self
                .monitors
                .contains_key(&(change.origin, change.dir.clone()))
            {
                self.coalescer.record(change, now);
            }
        }
        self.coalescer.take_due(now)
    }

    /// Puts a change back to be retried later (its panel is still loading).
    pub fn defer(&mut self, change: DirChange, now: Instant) {
        self.coalescer.record(change, now);
    }

    /// Sender used by the monitors (tests inject changes through it).
    #[cfg(test)]
    fn sender(&self) -> Sender<DirChange> {
        self.tx.clone()
    }
}
