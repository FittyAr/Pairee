use crate::plugin::registry::{emit_hook_event, get_loaded_plugins};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};

/// Process-wide cache of the current Secure-Mode flag. The main
/// loop refreshes this on every tick (via `set_secure_mode_cached`)
/// so that `emit_event` can read it without hitting the
/// filesystem on every keystroke — `on_key` fires per keypress
/// and a per-call `AppConfig::load_or_create` would do a
/// synchronous TOML read on every single key the user presses.
static SECURE_MODE_CACHED: OnceLock<AtomicBool> = OnceLock::new();

fn secure_mode_cached() -> &'static AtomicBool {
    SECURE_MODE_CACHED.get_or_init(|| AtomicBool::new(false))
}

/// Update the cached Secure-Mode flag. Called by the main loop
/// after the `AppConfig` is reloaded so the broadcast filter sees
/// the latest value.
pub fn set_secure_mode_cached(value: bool) {
    secure_mode_cached().store(value, Ordering::Relaxed);
}

/// Broadcasts a hook event (e.g. "on_cd", "on_hover", "on_key") to
/// all loaded plugins.
///
/// §6 Secure-Mode filter: events whose payload could carry
/// sensitive keystrokes (today: `on_key`) are **not** delivered to
/// untrusted plugins while Secure Mode is active. The trust flag
/// is recorded on `PluginInfo` at registration time so we can
/// filter here without consulting the global config again.
pub async fn emit_event(event_name: &str, data: serde_json::Value) {
    let data_str = data.to_string();
    let plugins = get_loaded_plugins().await;

    let secure_mode = secure_mode_cached().load(Ordering::Relaxed);

    for plugin in plugins {
        // Sensitive events are filtered to trusted plugins in
        // Secure Mode. Plugins are free to subscribe to non-key
        // events (e.g. `on_cd`, `on_hover`); the only payload that
        // is a known exfiltration vector is raw keystrokes.
        if secure_mode && !plugin.trusted && is_sensitive_event(event_name) {
            continue;
        }
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

/// Returns `true` for hook events whose payload may carry
/// sensitive user input. The list is intentionally small — we
/// only filter events that are known to contain user keystrokes.
fn is_sensitive_event(event_name: &str) -> bool {
    matches!(event_name, "on_key")
}
