//! Plugin manager: cross-thread request envelope, lifecycle, and
//! routing. The file was originally a single 750-line god module;
//! per AGENTS.md §1 it is now split across focused submodules:
//!
//! - `snapshot` — `AppStateSnapshot`, `FileEntrySnapshot` (the values that
//!   cross the mpsc channel toward plugins).
//! - `request` — `PluginRequest` enum + the structured payload structs
//!   (`DialogPosition`, `WhichCandidate`, `NotifyPayload`,
//!   `InputDialogResult`).
//! - `manager` — `PluginManager` (channel init, eager discovery) and the
//!   static `PLUGIN_REQ_TX/RX` channels.
//! - `dispatcher` — the main-loop `process_plugin_requests` function that
//!   routes every `PluginRequest` variant to its side effect.
//! - `dispatch_actions` — the side-effect helpers themselves
//!   (`render_notify`, `dispatch_emit_action`, `compute_file_cache_path`).
//!
//! Everything below is re-exported so the existing public path
//! `crate::plugin::manager::{PluginRequest, PluginManager,
//! process_plugin_requests, …}` keeps working unchanged.

pub mod dispatch_actions;
pub mod dispatcher;
pub mod manager;
pub mod request;
pub mod snapshot;

pub use manager::PluginManager;
pub use request::{
    DialogPosition, InputDialogResult, NotifyPayload, PluginRequest, WhichCandidate,
};
// `FileEntrySnapshot` is intentionally not re-exported here: it is only
// used internally by `dispatcher.rs` to build `AppStateSnapshot`. Plugins
// see the snapshot through `pairee.sync` and never need to name the
// type directly. Re-exporting it triggers a dead-code warning.

pub use dispatcher::process_plugin_requests;
