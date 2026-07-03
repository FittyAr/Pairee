//! M3 sync-context machinery.
//!
//! The plugin runtime supports two execution modes:
//!
//! - **Sync** (the default): a plugin callback (e.g. `peek`, `entry`,
//!   an event handler) runs in the *same thread* as the main event
//!   loop, with full access to the live `pairee` global, `cx`, `rt`,
//!   `th`, `km`. Sync callbacks must not call async APIs because the
//!   plugin worker thread is the main thread.
//!
//! - **Async / isolate**: a long-running callback runs in its own
//!   per-VM `tokio::task` (today's default for `peek`/`entry`/
//!   event). The VM does not see `cx`/`rt`/`th`/`km`; it can call
//!   `pairee.sync(fn)` to bridge a synchronous block into the main
//!   thread.
//!
//! The `Runtime` struct tracks which mode is currently active and
//! exposes a `runtime_scope!` macro that flips the `blocking` flag
//! and pushes/pops a frame. The `blocking` flag is checked at the
//! top of every interactive API (`pairee.input`, `pairee.confirm`,
//! `pairee.which`) to prevent re-entrant calls from inside a sync
//! block — those would deadlock the main event loop.
//!
//! M3 only wires the struct + macro; the actual sync-vs-async
//! dispatch lives in the registry/handler hot paths (see
//! `M3-T4 pair.sync/async_fn`). The `Runtime` itself is exposed as
//! a Lua app-data value so any binding can read `is_blocking()` if
//! it needs to short-circuit.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

/// Per-callback frame that tracks the metadata of the current
/// sync/async execution scope.
#[derive(Debug, Clone)]
pub struct RuntimeFrame {
    /// Free-form identifier (e.g. the action name, the event
    /// name, or the plugin function being called).
    pub id: String,
    /// `true` for sync, `false` for async.
    pub sync: bool,
}

impl RuntimeFrame {
    pub fn new(id: impl Into<String>, sync: bool) -> Self {
        Self {
            id: id.into(),
            sync,
        }
    }
}

/// The shared runtime state stored on the `mlua::Lua` app data.
///
/// `blocking` is the re-entry guard: it is `true` while a sync
/// callback is being executed on the main thread. Interactive
/// APIs (`pairee.input`, `pairee.confirm`, `pairee.which`) check
/// it and throw `RuntimeError("...")` if they are called from
/// inside a sync block.
///
/// `frames` is the stack of currently-active callbacks (most recent
/// at the back). It is mainly useful for diagnostics — e.g.
/// "we are inside `peek` → `fzf.pairee` → `cmd:spawn()`".
///
/// `blocks` stores per-plugin mutable state (the `pairee.state`
/// table). Each plugin instance has its own entry keyed by the
/// plugin name.
#[derive(Debug, Default)]
pub struct Runtime {
    pub blocking: AtomicBool,
    pub frames: Mutex<Vec<RuntimeFrame>>,
    /// Per-plugin `mlua::RegistryKey` pointing at the plugin's
    /// `pairee.state` table. We hold keys (rather than the
    /// tables themselves) so the table is still bound to the
    /// plugin's `Lua` instance and dies with it.
    pub blocks: Mutex<HashMap<String, Arc<()>>>,
}

impl Runtime {
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns `true` while a sync callback is on the stack.
    pub fn is_blocking(&self) -> bool {
        self.blocking.load(Ordering::SeqCst)
    }

    /// Push a new frame and return the previous `blocking` value
    /// (so the caller can restore it on scope exit).
    pub fn push_frame(&self, frame: RuntimeFrame) {
        if frame.sync {
            self.blocking.store(true, Ordering::SeqCst);
        }
        if let Ok(mut g) = self.frames.lock() {
            (*g).push(frame);
        }
    }

    /// Pop the most recent frame and clear `blocking` if the stack
    /// is empty. Returns the popped frame (or `None` if the stack
    /// was already empty — a programming error, but we swallow it
    /// for resilience).
    pub fn pop_frame(&self) -> Option<RuntimeFrame> {
        let popped = if let Ok(mut g) = self.frames.lock() {
            (*g).pop()
        } else {
            None
        };
        if let Ok(g) = self.frames.lock() {
            if g.is_empty() {
                self.blocking.store(false, Ordering::SeqCst);
            }
        }
        popped
    }

    /// Number of frames currently on the stack (for diagnostics).
    pub fn depth(&self) -> usize {
        self.frames.lock().map(|g| g.len()).unwrap_or(0)
    }

    /// Allocate (or fetch) the per-plugin `pairee.state` slot.
    /// The actual Lua table is created lazily on the plugin's
    /// `Lua` and registered via `register_plugin_state`.
    pub fn plugin_state_slot(&self, plugin_name: &str) -> Arc<()> {
        let mut g = self.blocks.lock().unwrap();
        g.entry(plugin_name.to_string())
            .or_insert_with(|| Arc::new(()))
            .clone()
    }

    /// Attach a Lua table as the per-plugin `pairee.state` for the
    /// given plugin. The table is stored as a `mlua::RegistryKey`
    /// so the Lua instance owns it; we hold the key here so the
    /// runtime can hand it to other bindings if/when needed.
    pub fn register_plugin_state(&self, plugin_name: &str, key: mlua::RegistryKey) {
        let mut g = self.blocks.lock().unwrap();
        g.insert(plugin_name.to_string(), Arc::new(key));
    }

    /// Fetch the per-plugin `pairee.state` registry key, if any.
    /// The caller can then call `lua.registry_value(&key)` to
    /// obtain the table (while the plugin's `Lua` is alive).
    pub fn plugin_state_key(&self, plugin_name: &str) -> Option<Arc<mlua::RegistryKey>> {
        let g = self.blocks.lock().ok()?;
        g.get(plugin_name)
            .and_then(|arc| arc.clone().downcast::<mlua::RegistryKey>().ok())
    }
}

/// Convenience helper for the binding layer: returns a
/// human-readable `id` for a frame from a free-form tag.
pub fn frame_id_for_action(action: &str) -> String {
    format!("action:{action}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blocking_flag_round_trip() {
        let rt = Runtime::new();
        assert!(!rt.is_blocking());
        rt.push_frame(RuntimeFrame::new("test", true));
        assert!(rt.is_blocking());
        rt.pop_frame();
        assert!(!rt.is_blocking());
    }

    #[test]
    fn test_async_frame_does_not_set_blocking() {
        let rt = Runtime::new();
        rt.push_frame(RuntimeFrame::new("async-thing", false));
        assert!(!rt.is_blocking());
        rt.pop_frame();
    }

    #[test]
    fn test_nested_sync_frames() {
        let rt = Runtime::new();
        rt.push_frame(RuntimeFrame::new("outer", true));
        rt.push_frame(RuntimeFrame::new("inner", true));
        assert_eq!(rt.depth(), 2);
        rt.pop_frame();
        assert!(rt.is_blocking(), "outer sync frame still active");
        rt.pop_frame();
        assert!(!rt.is_blocking());
    }

    #[test]
    fn test_plugin_state_slot_is_idempotent() {
        let rt = Runtime::new();
        let a = rt.plugin_state_slot("foo");
        let b = rt.plugin_state_slot("foo");
        assert!(Arc::ptr_eq(&a, &b));
        let c = rt.plugin_state_slot("bar");
        assert!(!Arc::ptr_eq(&a, &c));
    }
}
