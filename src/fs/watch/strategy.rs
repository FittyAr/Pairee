//! How a folder is observed: native notifications or polling (Strategy).

use super::monitor::ChangeSink;
use super::poller::PollStrategy;
use super::watcher::WatchStrategy;
use std::path::Path;
use std::sync::mpsc::Receiver;
use std::time::Duration;

/// Observes one folder and reports its changes.
pub trait ChangeStrategy {
    /// Reports changes of `dir` to `sink` until `stop` fires or its sender
    /// is dropped. `Err` when the strategy cannot monitor `dir`; the next
    /// strategy of the chain is tried then.
    fn run(&self, dir: &Path, stop: &Receiver<()>, sink: &ChangeSink) -> Result<(), String>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StrategyKind {
    /// Native notifications (`notify`), non-recursive.
    Watch,
    /// Folder signature (modification time + entry count) checked on an
    /// interval.
    Poll,
}

impl StrategyKind {
    pub fn build(self, poll_interval: Duration) -> Box<dyn ChangeStrategy> {
        match self {
            Self::Watch => Box::new(WatchStrategy),
            Self::Poll => Box::new(PollStrategy::new(poll_interval)),
        }
    }
}

/// Strategies to try in order. Polling is used directly for folders that
/// asked for it or live on file systems without reliable notifications,
/// and as the fallback when a watch cannot be set up.
pub fn strategy_chain(prefer_poll: bool, needs_polling: bool) -> Vec<StrategyKind> {
    if prefer_poll || needs_polling {
        vec![StrategyKind::Poll]
    } else {
        vec![StrategyKind::Watch, StrategyKind::Poll]
    }
}
