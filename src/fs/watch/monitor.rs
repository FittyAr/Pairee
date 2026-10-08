//! One monitored folder: a thread running the strategy chain until the
//! [`DirMonitor`] handle is dropped.

use super::strategy::strategy_chain;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::{Duration, SystemTime};

/// Something changed in `dir`. `entries` lists the changed children when
/// the strategy knows them (empty: unknown, e.g. after a poll).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirChange {
    pub dir: PathBuf,
    pub entries: Vec<PathBuf>,
}

impl DirChange {
    /// A change whose entries are unknown.
    pub fn whole(dir: PathBuf) -> Self {
        Self {
            dir,
            entries: Vec::new(),
        }
    }
}

/// Observer end handed to every monitor: forwards changes to the event loop
/// and wakes it.
#[derive(Clone)]
pub struct ChangeSink {
    tx: Sender<DirChange>,
    wake: fn(),
    /// Folder changes from this moment on may predate the monitor.
    since: Option<SystemTime>,
}

impl ChangeSink {
    pub fn new(tx: Sender<DirChange>, wake: fn()) -> Self {
        Self {
            tx,
            wake,
            since: None,
        }
    }

    pub fn report(&self, change: DirChange) {
        if self.tx.send(change).is_ok() {
            (self.wake)();
        }
    }

    /// Called by a strategy once it observes `dir`. A change made while
    /// the monitor was being set up (after the panel read the folder)
    /// produced no event, so a folder modified since shortly before the
    /// monitor started is reread once.
    pub fn armed(&self, dir: &Path) {
        let modified = std::fs::metadata(dir).and_then(|m| m.modified());
        if let (Some(since), Ok(modified)) = (self.since, modified)
            && modified >= since
        {
            self.report(DirChange::whole(dir.to_path_buf()));
        }
    }
}

/// How long before a monitor starts a folder change still counts as
/// possibly missed (the panel's listing precedes the monitor).
const ARM_GRACE: Duration = Duration::from_secs(2);

/// How a folder should be monitored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MonitorPlan {
    /// Poll instead of watching (e.g. the folder holds more objects than
    /// the automatic update limit).
    pub prefer_poll: bool,
    pub poll_interval: Duration,
}

/// Handle of a running monitor; dropping it stops the thread.
pub struct DirMonitor {
    _stop: Sender<()>,
}

impl DirMonitor {
    /// Starts monitoring `dir`. File system checks that may block (network
    /// mounts) and the watch set-up run on the monitor thread.
    pub fn start(dir: PathBuf, plan: MonitorPlan, mut sink: ChangeSink) -> Self {
        sink.since = SystemTime::now().checked_sub(ARM_GRACE);
        let (stop_tx, stop_rx) = channel();
        let spawned = std::thread::Builder::new()
            .name("pairee-watch".into())
            .spawn(move || run_chain(&dir, plan, &stop_rx, &sink));
        if let Err(err) = spawned {
            log::warn!("Auto-refresh: cannot start a monitor thread: {err}");
        }
        Self { _stop: stop_tx }
    }
}

/// Runs the strategies in order; the next one is tried when one cannot
/// monitor the folder.
fn run_chain(dir: &std::path::Path, plan: MonitorPlan, stop: &Receiver<()>, sink: &ChangeSink) {
    let chain = strategy_chain(plan.prefer_poll, super::needs_polling(dir));
    for kind in chain {
        match kind.build(plan.poll_interval).run(dir, stop, sink) {
            Ok(()) => return,
            Err(err) => log::info!("Auto-refresh: {kind:?} unavailable for {dir:?}: {err}"),
        }
    }
}
