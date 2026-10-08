//! Directory synchronization core (Total Commander "Synchronize dirs").
//!
//! * [`walk::diff_trees`] compares two local trees recursively (size + time
//!   with a tolerance, optionally content hashes) honouring a filter mask.
//! * [`plan`] picks an action per difference from the [`SyncDirection`],
//!   lets the user change it and groups the result into Transfer Engine
//!   copy/delete jobs — the copying itself is the engine's.
//!
//! Nothing here depends on the UI or on Tokio.

pub mod model;
pub mod options;
pub mod plan;
pub mod walk;

#[cfg(test)]
mod tests;

pub use model::{DiffKind, SyncAction, SyncDirection, SyncItem, compare_entries};
pub use options::SyncOptions;
pub use plan::{SyncSummary, plan_jobs};
pub use walk::{ScanObserver, ScanProgress, SyncError, diff_trees};
