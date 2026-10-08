//! One monitored folder: a thread running the strategy chain until the
//! [`DirMonitor`] handle is dropped.

use super::poller::PollStrategy;
use super::strategy::{ChangeStrategy, strategy_chain};
use crate::fs::vfs::Vfs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::{Duration, SystemTime};

/// Which filesystem a monitored folder is on: the local disk, or the SFTP
/// connection with the given identity (`SharedSshClient::id`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum WatchOrigin {
    #[default]
    Local,
    Remote(usize),
}

/// Something changed in `dir`. `entries` lists the changed children when
/// the strategy knows them (empty: unknown, e.g. after a poll).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirChange {
    pub dir: PathBuf,
    pub entries: Vec<PathBuf>,
    /// Set by the [`ChangeSink`] that reports it.
    pub origin: WatchOrigin,
}

impl DirChange {
    /// A change whose entries are unknown.
    pub fn whole(dir: PathBuf) -> Self {
        Self {
            dir,
            entries: Vec::new(),
            origin: WatchOrigin::Local,
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
    origin: WatchOrigin,
}

impl ChangeSink {
    pub fn new(tx: Sender<DirChange>, wake: fn()) -> Self {
        Self {
            tx,
            wake,
            since: None,
            origin: WatchOrigin::Local,
        }
    }

    /// The sink of a monitor on `origin`; its changes carry it.
    pub fn with_origin(mut self, origin: WatchOrigin) -> Self {
        self.origin = origin;
        self
    }

    pub fn report(&self, mut change: DirChange) {
        change.origin = self.origin;
        if self.tx.send(change).is_ok() {
            (self.wake)();
        }
    }

    /// Called by a strategy once it observes `dir`. A change made while
    /// the monitor was being set up (after the panel read the folder)
    /// produced no event, so a folder modified since shortly before the
    /// monitor started is reread once.
    pub fn armed(&self, dir: &Path) {
        self.armed_at(dir, std::fs::metadata(dir).and_then(|m| m.modified()).ok());
    }

    /// [`Self::armed`] with the folder time the strategy already read.
    pub fn armed_at(&self, dir: &Path, modified: Option<SystemTime>) {
        if let (Some(since), Some(modified)) = (self.since, modified)
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
    pub fn start(dir: PathBuf, plan: MonitorPlan, sink: ChangeSink) -> Self {
        Self::spawn(sink, move |stop, sink| run_chain(&dir, plan, stop, sink))
    }

    /// Polls `dir` of `vfs` (an SFTP server) every `interval`; the checks
    /// run on the monitor thread, never on the interface thread.
    pub fn start_polling(
        dir: PathBuf,
        vfs: Arc<dyn Vfs>,
        interval: Duration,
        sink: ChangeSink,
    ) -> Self {
        Self::spawn(sink, move |stop, sink| {
            if let Err(err) = PollStrategy::over(vfs, interval).run(&dir, stop, sink) {
                log::info!("Auto-refresh: cannot poll {dir:?}: {err}");
            }
        })
    }

    fn spawn(
        mut sink: ChangeSink,
        body: impl FnOnce(&Receiver<()>, &ChangeSink) + Send + 'static,
    ) -> Self {
        sink.since = SystemTime::now().checked_sub(ARM_GRACE);
        let (stop_tx, stop_rx) = channel();
        let spawned = std::thread::Builder::new()
            .name("pairee-watch".into())
            .spawn(move || body(&stop_rx, &sink));
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
