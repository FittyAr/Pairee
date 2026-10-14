//! Events delivered to the plugins' `pairee.ps.sub` callbacks: host hooks
//! (`on_cd`, `on_hover`, `on_key`, `on_mouse`) and `pairee.ps.pub` messages
//! of other plugins. The sender is recorded by Rust, never by Lua, so a
//! subscriber can tell host events from plugin messages and which plugin
//! sent one.

use crate::plugin::manager::PluginOrigin;
use crate::plugin::registry::{emit_hook_event, get_loaded_plugins};

/// Prefixes of event names only the host (or a trusted plugin) may publish:
/// the host hooks (`on_*`) and anything under `pairee.`.
const RESERVED_PREFIXES: [&str; 2] = ["on_", "pairee."];

/// Whether `event` is a host / system event name an untrusted plugin may
/// not publish.
pub fn is_reserved(event: &str) -> bool {
    RESERVED_PREFIXES
        .iter()
        .any(|prefix| event.starts_with(prefix))
}

/// Broadcasts a host hook event (e.g. "on_cd", "on_hover", "on_key") to all
/// loaded plugins.
pub async fn emit_event(event_name: &str, data: serde_json::Value) {
    broadcast(event_name, &data.to_string(), None).await;
}

/// Broadcasts `pairee.ps.pub(event_name, data)` of the plugin `sender`
/// (`data` is the JSON text of the payload).
pub async fn publish(sender: PluginOrigin, event_name: &str, data: &str) {
    broadcast(event_name, data, Some(sender)).await;
}

/// Queues the event for every loaded plugin without waiting: a plugin
/// whose queue is full (busy or stuck) misses it, so a flood of events
/// never piles up tasks or copies of the payload in the host.
async fn broadcast(event_name: &str, data: &str, sender: Option<PluginOrigin>) {
    for plugin in get_loaded_plugins().await {
        // The Lua side checks whether the plugin has callbacks for the event.
        emit_hook_event(
            &plugin.manifest.name,
            event_name,
            data.to_string(),
            sender.clone(),
        )
        .await;
    }
}

#[cfg(test)]
mod tests {
    use super::is_reserved;

    #[test]
    fn host_and_system_names_are_reserved() {
        for name in [
            "on_cd",
            "on_hover",
            "on_key",
            "on_mouse",
            "on_anything",
            "pairee.quit",
        ] {
            assert!(is_reserved(name), "{name}");
        }
        for name in ["git-status", "my_plugin.done", "onion", "paireex"] {
            assert!(!is_reserved(name), "{name}");
        }
    }
}
