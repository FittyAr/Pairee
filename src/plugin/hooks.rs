use crate::plugin::registry::{emit_hook_event, get_loaded_plugins};

/// Broadcasts a hook event (e.g. "on_cd", "on_hover", "on_key") to all loaded plugins.
pub async fn emit_event(event_name: &str, data: serde_json::Value) {
    let data_str = data.to_string();
    let plugins = get_loaded_plugins().await;
    for plugin in plugins {
        // Let's emit the hook event to every active plugin task.
        // The Lua side registry checks if the plugin VM has callbacks registered for this event.
        let name = plugin.manifest.name.clone();
        let ev = event_name.to_string();
        let d = data_str.clone();
        tokio::spawn(async move {
            emit_hook_event(&name, &ev, d).await;
        });
    }
}
