//! Directory change monitoring for panel auto-refresh.
//!
//! A [`DirMonitor`] watches one folder on its own thread and reports to a
//! [`ChangeSink`] (Observer): the event loop drains the sink, feeds a
//! [`Coalescer`] that turns bursts into one refresh per folder, and rereads
//! the panels showing it.
//!
//! How a folder is observed is a Strategy ([`ChangeStrategy`]): native
//! notifications ([`StrategyKind::Watch`], `notify`, non-recursive) or a
//! cheap signature poll ([`StrategyKind::Poll`]) for network and other
//! file systems without reliable notifications, very large folders, and as
//! the fallback when a watch cannot be set up.

mod coalesce;
mod monitor;
mod poller;
mod strategy;
mod support;
mod watcher;

#[cfg(test)]
mod tests;

pub use coalesce::{CoalesceTiming, Coalescer};
pub use monitor::{ChangeSink, DirChange, DirMonitor, MonitorPlan, WatchOrigin};
pub use support::needs_polling;
