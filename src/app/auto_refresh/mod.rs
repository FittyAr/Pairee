//! Panel auto-refresh: keeps one [`DirMonitor`] per folder shown by a
//! visible local tab, re-arming them as tabs move or close, and hands the
//! coalesced changes to [`AppState::poll_auto_refresh`] (see `apply.rs`).
//!
//! [`AppState::poll_auto_refresh`]: crate::app::state::AppState::poll_auto_refresh

mod apply;
#[cfg(test)]
mod tests;

use crate::config::settings::Settings;
use crate::fs::watch::{ChangeSink, CoalesceTiming, Coalescer, DirChange, DirMonitor, MonitorPlan};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::{Duration, Instant};

/// A folder that should be monitored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WatchTarget {
    pub dir: PathBuf,
    /// Poll instead of watching (the folder exceeds the update limit).
    pub prefer_poll: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Config {
    enabled: bool,
    poll_interval: Duration,
}

impl Config {
    fn from_settings(settings: &Settings) -> Self {
        Self {
            enabled: settings.auto_refresh,
            poll_interval: Duration::from_secs(u64::from(settings.auto_refresh_poll_secs.max(1))),
        }
    }
}

pub struct AutoRefresh {
    config: Config,
    monitors: HashMap<PathBuf, (MonitorPlan, DirMonitor)>,
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

    /// Monitors exactly `targets` (none when disabled): monitors of folders
    /// no longer shown stop, new folders start being monitored.
    pub fn sync(&mut self, targets: Vec<WatchTarget>) {
        let wanted: HashMap<PathBuf, MonitorPlan> = if self.config.enabled {
            targets
                .into_iter()
                .map(|t| (t.dir, self.plan(t.prefer_poll)))
                .collect()
        } else {
            HashMap::new()
        };
        self.monitors
            .retain(|dir, (plan, _)| wanted.get(dir) == Some(plan));
        for (dir, plan) in wanted {
            if !self.monitors.contains_key(&dir) {
                let sink = ChangeSink::new(self.tx.clone(), crate::app::jobs::wake_event_loop);
                let monitor = DirMonitor::start(dir.clone(), plan, sink);
                self.monitors.insert(dir, (plan, monitor));
            }
        }
        let monitors = &self.monitors;
        self.coalescer.retain(|dir| monitors.contains_key(dir));
    }

    fn plan(&self, prefer_poll: bool) -> MonitorPlan {
        MonitorPlan {
            prefer_poll,
            poll_interval: self.config.poll_interval,
        }
    }

    /// Folders currently monitored, sorted.
    pub fn monitored(&self) -> Vec<&Path> {
        let mut dirs: Vec<&Path> = self.monitors.keys().map(PathBuf::as_path).collect();
        dirs.sort();
        dirs
    }

    /// Collects the reported changes and returns the folders due at `now`.
    pub fn take_due(&mut self, now: Instant) -> Vec<DirChange> {
        while let Ok(change) = self.rx.try_recv() {
            if self.monitors.contains_key(&change.dir) {
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
