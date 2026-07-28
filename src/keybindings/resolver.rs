//! Resolves keypresses to actions with explicit source attribution.
//!
//! The resolver is the single source of truth for "which action
//! does this keypress trigger?" It loads bindings from four
//! sources, in increasing priority:
//!
//! 1. The active built-in preset (`norton`, `neovim`, `vscode`).
//! 2. The user's `keybindings.toml` `[custom_bindings]` overrides.
//! 3. Plugin manifest entries (`[keybindings]` in `manifest.toml`).
//! 4. Runtime Lua calls (`pairee.keybindings.bind`).
//!
//! Layers 1 + 2 are **immutable** after construction. Layers 3 + 4
//! live in a `RwLock` overlay and can be added / removed at runtime
//! (the plugin loader does this when it loads / unloads a plugin,
//! the Lua binding API does it on every `pairee.keybindings.bind`
//! call). The `RwLock` keeps the read path (the main loop fires
//! `resolve()` on every keypress) fast: a parked lock on the
//! plugin overlay is only paid when a plugin is being registered
//! or unloaded, not on every keystroke.
//!
//! See [`crate::keybindings::source`] for the priority table and
//! the per-source semantics.

use super::actions::Action;
use super::preset::{normalize_key_string, parse_action_name};
use super::source::{BindingSource, ConflictPolicy, RegisterOutcome, ResolvedBinding};
use crate::config::AppConfig;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::collections::HashMap;
use std::sync::RwLock;

/// Overlay for Plugin and Lua bindings. The two layers can be
/// added or removed at runtime; Builtin and User layers are
/// loaded once at construction and never change.
type PluginOverlay = HashMap<String, ResolvedBinding>;

pub struct KeybindingResolver {
    /// Layers 1 (preset) + 2 (user overrides). Read-only after
    /// construction.
    static_layers: HashMap<String, ResolvedBinding>,
    /// Layers 3 (plugin manifest) + 4 (Lua runtime). The
    /// `RwLock` allows the plugin loader and the Lua binding API
    /// to mutate without rebuilding the whole resolver.
    plugin_overlay: RwLock<PluginOverlay>,
    /// Cached `Action -> first key string` (excludes the
    /// `PluginCommand` sentinel). Recomputed from the union of
    /// both layers, under the same lock as `plugin_overlay`.
    inverse: RwLock<HashMap<Action, String>>,
}

impl KeybindingResolver {
    /// Build a resolver with the Builtin (preset) + User (custom)
    /// layers populated. Plugin and Lua layers are populated
    /// later through [`Self::register`] (and cleared with
    /// [`Self::unregister_plugin`]).
    pub fn new(config: &AppConfig) -> Self {
        let mut static_layers = HashMap::new();

        // Layer 1 — built-in preset.
        for (key, action) in super::preset::get_preset_bindings(&config.keybindings.preset) {
            static_layers.insert(normalize_key_string(&key), ResolvedBinding::builtin(action));
        }

        // Layer 2 — user overrides from `keybindings.toml`.
        for (action_name, key_str) in &config.keybindings.custom_bindings {
            let Some(action) = parse_action_name(action_name) else {
                log::warn!(
                    "keybindings.toml: unknown action '{}' — skipped",
                    action_name
                );
                continue;
            };
            for key in key_str.split(',') {
                let trimmed = key.trim();
                if trimmed.is_empty() {
                    continue;
                }
                let key = normalize_key_string(trimmed);
                static_layers.insert(
                    key,
                    ResolvedBinding {
                        action,
                        source: BindingSource::User,
                        plugin_action: String::new(),
                    },
                );
            }
        }

        let resolver = Self {
            static_layers,
            plugin_overlay: RwLock::new(HashMap::new()),
            inverse: RwLock::new(HashMap::new()),
        };
        resolver.rebuild_inverse();
        resolver
    }

    /// Resolves a `KeyEvent` into the binding that owns it, or
    /// `None` if the key is unbound.
    ///
    /// Read path: clones the binding out of the plugin overlay
    /// (one `RwLock` acquire, no mutation). On the hot path this
    /// is taken on every keystroke.
    pub fn resolve(&self, key_event: KeyEvent) -> Option<ResolvedBinding> {
        let key_str = key_event_to_string(key_event);
        if key_str.is_empty() {
            return None;
        }
        self.resolve_for_key_string(&key_str)
    }

    /// Resolves a canonical key string (e.g. `"F7"`, `"Alt+F5"`,
    /// `"ctrl+d"`) into the binding that owns it.
    pub fn resolve_for_key_string(&self, key: &str) -> Option<ResolvedBinding> {
        // Fast path: the plugin overlay is empty in the common
        // case (no plugin installed, or the installed plugin
        // declared no keybindings). Skip the RwLock entirely.
        //
        // We `try_read` instead of `read` so a writer doesn't
        // block the keypress; if a writer holds the lock right
        // now, we fall through to the static layer and let the
        // next keystroke pick up the plugin's binding.
        if let Ok(overlay) = self.plugin_overlay.try_read() {
            if let Some(b) = overlay.get(key) {
                return Some(b.clone());
            }
        }
        self.static_layers.get(key).cloned()
    }

    /// First key string bound to `action` in the active preset
    /// (Builtin + User) and in the plugin overlay, ignoring the
    /// `PluginCommand` sentinel. Used by the F-key bar and the
    /// menu renderer.
    pub fn key_for_action(&self, action: Action) -> Option<String> {
        let inverse = self.inverse.read().ok()?;
        inverse.get(&action).cloned()
    }

    /// Total number of bindings currently stored (across all
    /// layers). Useful for tests and for the diagnostics /
    /// "Conflicts" sub-screen the plugin menu will eventually
    /// show.
    pub fn len(&self) -> usize {
        let overlay = self.plugin_overlay.read().map(|o| o.len()).unwrap_or(0);
        self.static_layers.len() + overlay
    }

    /// Snapshot of every current binding (static layers + plugin
    /// overlay), sorted by key. Used by the diagnostics /
    /// "Conflicts" sub-screen.
    pub fn bindings(&self) -> Vec<(String, ResolvedBinding)> {
        let mut out: Vec<(String, ResolvedBinding)> = Vec::with_capacity(self.len());
        for (k, v) in &self.static_layers {
            out.push((k.clone(), v.clone()));
        }
        if let Ok(overlay) = self.plugin_overlay.read() {
            for (k, v) in overlay.iter() {
                out.push((k.clone(), v.clone()));
            }
        }
        out.sort_by(|a, b| a.0.cmp(&b.0));
        out
    }

    /// Register a binding, applying the priority + conflict rules
    /// from [`super::source`]. Used by the plugin registry
    /// (manifest entries) and by the Lua runtime API.
    ///
    /// Returns the outcome so the caller can log or surface a
    /// toast. The resolver never panics.
    ///
    /// Builtin and User layers cannot be modified through this
    /// method — they are immutable after construction. Calling
    /// `register` with a `BindingSource::Builtin` or
    /// `BindingSource::User` is rejected as `Invalid`.
    pub fn register(
        &self,
        key_str: &str,
        binding: ResolvedBinding,
        policy: ConflictPolicy,
    ) -> RegisterOutcome {
        // The plugin overlay is the only mutable layer.
        if matches!(binding.source, BindingSource::Builtin | BindingSource::User) {
            return RegisterOutcome::Invalid;
        }
        let key = normalize_key_string(key_str);
        if key.is_empty() {
            return RegisterOutcome::Invalid;
        }

        let mut overlay = match self.plugin_overlay.write() {
            Ok(g) => g,
            Err(_) => {
                log::error!("keybindings: plugin_overlay RwLock poisoned");
                return RegisterOutcome::Invalid;
            }
        };

        // The new binding always goes into the overlay; the
        // static layers are checked separately to decide
        // "Conflict" vs "Bound". User bindings in the static
        // layer are FINAL: even `Override` cannot displace
        // them, because the user is the ultimate authority and
        // a plugin that wants to take a User-bound key has to
        // ask the user to remove the override first.
        let existing: Option<ResolvedBinding> = if let Some(b) = overlay.get(&key).cloned() {
            Some(b)
        } else if let Some(b) = self.static_layers.get(&key).cloned() {
            // The static layer can only be displaced when the
            // entry is a Builtin. User bindings are reported
            // back as "existing" so the policy can return
            // Conflict under every code path; the policy
            // itself never re-binds a User source.
            Some(b)
        } else {
            None
        };

        match existing {
            None => {
                overlay.insert(key, binding);
                drop(overlay);
                self.rebuild_inverse();
                RegisterOutcome::Bound
            }
            Some(existing) => {
                let is_user = matches!(existing.source, BindingSource::User);
                let new_pri = binding.source.priority();
                let existing_pri = existing.source.priority();

                // A User binding in the static layer is
                // permanent; neither policy can displace it.
                if is_user {
                    log::debug!(
                        "keybindings: refusing to bind '{}' ({}): already user-owned",
                        key,
                        binding.source
                    );
                    return RegisterOutcome::Conflict {
                        with: existing.source.clone(),
                    };
                }

                match policy {
                    ConflictPolicy::Fallback => {
                        // The previous semantics (priority-based
                        // comparison) were wrong: a plugin asking
                        // for Fallback means "I will only take
                        // the key if nobody else wants it." The
                        // existence of any binding — Builtin or
                        // Plugin — disqualifies the request,
                        // regardless of the priority numbers.
                        log::debug!(
                            "keybindings: refusing to bind '{}' ({}): already owned by {}",
                            key,
                            binding.source,
                            existing.source
                        );
                        RegisterOutcome::Conflict {
                            with: existing.source.clone(),
                        }
                    }
                    ConflictPolicy::Override => {
                        // Override still respects the priority
                        // table: a lower-priority source cannot
                        // take a key from a higher-priority
                        // source. (Today no source has a higher
                        // priority than Lua, so the only
                        // practical case is "Plugin > Builtin
                        // via Override".) The priority check is
                        // kept so a future lower-priority source
                        // (e.g. an "auto-suggested" Plugin mode)
                        // cannot quietly hijack a key.
                        if new_pri <= existing_pri {
                            log::debug!(
                                "keybindings: refusing to override '{}' ({} -> {}): \
                                 existing has equal or higher priority",
                                key,
                                existing.source,
                                binding.source
                            );
                            return RegisterOutcome::Conflict {
                                with: existing.source.clone(),
                            };
                        }
                        log::info!(
                            "keybindings: override '{}' ({} -> {})",
                            key,
                            existing.source,
                            binding.source
                        );
                        overlay.insert(key, binding);
                        drop(overlay);
                        self.rebuild_inverse();
                        RegisterOutcome::Bound
                    }
                }
            }
        }
    }

    /// Unregister every binding that came from a specific
    /// plugin (manifest or Lua). Used when a plugin is unloaded
    /// to free the keys it occupied. The resolver never touches
    /// the static layers, so a user override is preserved.
    pub fn unregister_plugin(&self, plugin: &str) {
        let mut overlay = match self.plugin_overlay.write() {
            Ok(g) => g,
            Err(_) => {
                log::error!("keybindings: plugin_overlay RwLock poisoned");
                return;
            }
        };
        let before = overlay.len();
        overlay.retain(|_, b| match &b.source {
            BindingSource::Plugin { plugin: p } | BindingSource::Lua { plugin: p } => p != plugin,
            BindingSource::Builtin | BindingSource::User => true,
        });
        let released = before - overlay.len();
        drop(overlay);
        if released > 0 {
            log::info!(
                "keybindings: released {} binding(s) for plugin '{}'",
                released,
                plugin
            );
            self.rebuild_inverse();
        }
    }

    /// Snapshot the plugin overlay as `(key, plugin_name,
    /// plugin_action)` tuples. The plugin registry keeps a
    /// parallel copy in its own `OnceLock` storage; this method
    /// is the canonical source of truth for "what does the
    /// resolver know about plugin bindings right now?". The
    /// plugin registry can use it to verify its own cache
    /// stays in sync, and a future "Conflicts" sub-screen can
    /// use it directly.
    #[cfg(test)]
    pub fn plugin_bindings(&self) -> Vec<(String, String, String)> {
        let overlay = match self.plugin_overlay.read() {
            Ok(o) => o,
            Err(_) => return Vec::new(),
        };
        overlay
            .iter()
            .filter_map(|(k, b)| match &b.source {
                BindingSource::Plugin { plugin } | BindingSource::Lua { plugin } => {
                    Some((k.clone(), plugin.clone(), b.plugin_action.clone()))
                }
                _ => None,
            })
            .collect()
    }

    fn rebuild_inverse(&self) {
        let mut inverse: HashMap<Action, String> = HashMap::new();
        for (key, binding) in &self.static_layers {
            if matches!(binding.action, Action::PluginCommand) {
                continue;
            }
            inverse.entry(binding.action).or_insert_with(|| key.clone());
        }
        if let Ok(overlay) = self.plugin_overlay.read() {
            for (key, binding) in overlay.iter() {
                if matches!(binding.action, Action::PluginCommand) {
                    continue;
                }
                inverse.entry(binding.action).or_insert_with(|| key.clone());
            }
        }
        match self.inverse.write() {
            Ok(mut g) => *g = inverse,
            Err(e) => {
                log::error!("keybindings: inverse RwLock poisoned: {}", e);
            }
        }
    }
}

/// Converts a crossterm `KeyEvent` into a standard
/// human-readable string representation. Examples: `"Ctrl+H"`,
/// `"F5"`, `"Alt+F7"`, `"Shift+F9"`, `"Ctrl+Alt+1"`, `"Gray+"`.
pub fn key_event_to_string(key: KeyEvent) -> String {
    let has_ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    let has_alt = key.modifiers.contains(KeyModifiers::ALT);
    let has_shift = key.modifiers.contains(KeyModifiers::SHIFT);

    let code_str = match key.code {
        KeyCode::Char(' ') => "Space".to_string(),
        KeyCode::Char(c) => {
            if has_shift && c.is_ascii_lowercase() {
                c.to_ascii_uppercase().to_string()
            } else {
                c.to_string()
            }
        }
        KeyCode::F(num) => format!("F{}", num),
        KeyCode::Up => "Up".to_string(),
        KeyCode::Down => "Down".to_string(),
        KeyCode::Left => "Left".to_string(),
        KeyCode::Right => "Right".to_string(),
        KeyCode::Tab => "Tab".to_string(),
        KeyCode::Enter => "Enter".to_string(),
        KeyCode::Backspace => "Backspace".to_string(),
        KeyCode::Delete => "Delete".to_string(),
        KeyCode::Insert => "Insert".to_string(),
        KeyCode::Esc => "Esc".to_string(),
        KeyCode::Home => "Home".to_string(),
        KeyCode::End => "End".to_string(),
        KeyCode::PageUp => "PageUp".to_string(),
        KeyCode::PageDown => "PageDown".to_string(),
        _ => String::new(),
    };

    if code_str.is_empty() {
        return String::new();
    }

    let mut parts: Vec<&str> = Vec::new();
    if has_ctrl {
        parts.push("Ctrl");
    }
    if has_alt {
        parts.push("Alt");
    }
    if has_shift {
        parts.push("Shift");
    }

    if parts.is_empty() {
        code_str
    } else if parts.len() == 1 && parts[0] == "Shift" && matches!(key.code, KeyCode::Char(_)) {
        code_str
    } else {
        format!("{}+{}", parts.join("+"), code_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keybindings::source::BindingSource;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn empty_config() -> AppConfig {
        AppConfig {
            settings: crate::config::settings::Settings::default(),
            theme: crate::config::theme::Theme::default(),
            keybindings: crate::config::keybindings::KeybindingsConfig::default(),
        }
    }

    #[test]
    fn resolve_returns_none_for_unbound_key() {
        let resolver = KeybindingResolver::new(&empty_config());
        let key = KeyEvent::new(KeyCode::F(20), KeyModifiers::empty());
        assert!(resolver.resolve(key).is_none());
    }

    #[test]
    fn builtin_preset_wins_when_no_override_and_no_plugin() {
        let resolver = KeybindingResolver::new(&empty_config());
        let f3 = KeyEvent::new(KeyCode::F(3), KeyModifiers::empty());
        let r = resolver.resolve(f3).expect("F3 is in the preset");
        assert!(r.is_builtin());
        assert_eq!(r.action, Action::View);
        assert!(r.plugin_action.is_empty());
    }

    #[test]
    fn user_override_in_keybindings_toml_takes_precedence_over_preset() {
        let mut cfg = empty_config();
        cfg.keybindings
            .custom_bindings
            .insert("move_up".to_string(), "F3".to_string());
        let resolver = KeybindingResolver::new(&cfg);
        let r = resolver
            .resolve(KeyEvent::new(KeyCode::F(3), KeyModifiers::empty()))
            .expect("F3 is bound");
        assert!(r.is_user());
        assert_eq!(r.action, Action::MoveUp);
    }

    #[test]
    fn plugin_registration_lands_when_key_is_free() {
        let resolver = KeybindingResolver::new(&empty_config());
        let key = "Alt+Shift+K".to_string();
        let outcome = resolver.register(
            &key,
            ResolvedBinding::plugin("disk-usage.pairee", "entry"),
            ConflictPolicy::Fallback,
        );
        assert_eq!(outcome, RegisterOutcome::Bound);
        let r = resolver
            .resolve_for_key_string(&normalize_key_string(&key))
            .expect("registered");
        assert!(r.is_plugin());
        assert_eq!(r.plugin_action, "entry");
        // Plugin bindings appear in the F-key bar / menu
        // inverse map only when their action is *not* the
        // `PluginCommand` sentinel; entry() is, so it is
        // excluded from the inverse map.
        assert!(resolver.key_for_action(Action::PluginCommand).is_none());
    }

    #[test]
    fn plugin_registration_with_fallback_policy_loses_to_builtin() {
        let resolver = KeybindingResolver::new(&empty_config());
        let outcome = resolver.register(
            "F3",
            ResolvedBinding::plugin("archive-inspect.pairee", "entry"),
            ConflictPolicy::Fallback,
        );
        assert!(matches!(outcome, RegisterOutcome::Conflict { .. }));
        let r = resolver
            .resolve(KeyEvent::new(KeyCode::F(3), KeyModifiers::empty()))
            .expect("still bound");
        assert!(r.is_builtin());
        assert_eq!(r.action, Action::View);
    }

    #[test]
    fn plugin_registration_with_override_policy_beats_builtin() {
        let resolver = KeybindingResolver::new(&empty_config());
        let outcome = resolver.register(
            "F3",
            ResolvedBinding::plugin("archive-inspect.pairee", "entry"),
            ConflictPolicy::Override,
        );
        assert_eq!(outcome, RegisterOutcome::Bound);
        let r = resolver
            .resolve(KeyEvent::new(KeyCode::F(3), KeyModifiers::empty()))
            .expect("now bound to plugin");
        assert!(r.is_plugin());
        assert_eq!(
            r.source,
            BindingSource::Plugin {
                plugin: "archive-inspect.pairee".into()
            }
        );
    }

    #[test]
    fn lua_runtime_registration_beats_plugin_manifest() {
        let resolver = KeybindingResolver::new(&empty_config());
        let key = "Alt+Shift+Q".to_string();
        let _ = resolver.register(
            &key,
            ResolvedBinding::plugin("a.pairee", "entry"),
            ConflictPolicy::Override,
        );
        let outcome = resolver.register(
            &key,
            ResolvedBinding::lua("b.pairee", "entry"),
            ConflictPolicy::Override,
        );
        assert_eq!(outcome, RegisterOutcome::Bound);
        let r = resolver
            .resolve_for_key_string(&normalize_key_string(&key))
            .expect("now bound by Lua");
        assert!(matches!(r.source, BindingSource::Lua { .. }));
    }

    #[test]
    fn user_toml_always_beats_plugin_and_lua() {
        let mut cfg = empty_config();
        cfg.keybindings
            .custom_bindings
            .insert("move_down".to_string(), "F3".to_string());
        let resolver = KeybindingResolver::new(&cfg);
        // A plugin tries to claim F3; it should be rejected
        // because the User binding is in the static layer and
        // Plugin priority (60) is below User priority (100).
        let outcome = resolver.register(
            "F3",
            ResolvedBinding::plugin("x.pairee", "entry"),
            ConflictPolicy::Override,
        );
        assert!(matches!(outcome, RegisterOutcome::Conflict { .. }));
        let r = resolver
            .resolve(KeyEvent::new(KeyCode::F(3), KeyModifiers::empty()))
            .expect("still user-bound");
        assert!(r.is_user());
    }

    #[test]
    fn unregister_plugin_releases_only_its_keys() {
        let resolver = KeybindingResolver::new(&empty_config());
        let _ = resolver.register(
            "Alt+Shift+K",
            ResolvedBinding::plugin("disk-usage.pairee", "entry"),
            ConflictPolicy::Override,
        );
        let _ = resolver.register(
            "Alt+Shift+L",
            ResolvedBinding::plugin("other.pairee", "entry"),
            ConflictPolicy::Override,
        );
        let _ = resolver.register(
            "Alt+Shift+M",
            ResolvedBinding::lua("disk-usage.pairee", "entry"),
            ConflictPolicy::Override,
        );

        resolver.unregister_plugin("disk-usage.pairee");
        assert!(
            resolver
                .resolve_for_key_string(&normalize_key_string("Alt+Shift+K"))
                .is_none()
        );
        assert!(
            resolver
                .resolve_for_key_string(&normalize_key_string("Alt+Shift+M"))
                .is_none()
        );
        assert!(
            resolver
                .resolve_for_key_string(&normalize_key_string("Alt+Shift+L"))
                .is_some()
        );
    }

    #[test]
    fn register_with_empty_key_string_returns_invalid() {
        let resolver = KeybindingResolver::new(&empty_config());
        let outcome = resolver.register(
            "",
            ResolvedBinding::plugin("x.pairee", "entry"),
            ConflictPolicy::Override,
        );
        assert_eq!(outcome, RegisterOutcome::Invalid);
    }

    #[test]
    fn register_with_builtin_source_returns_invalid() {
        // The static layers are immutable after construction;
        // trying to mutate them through `register` is a
        // programming error. We return Invalid rather than
        // panicking so a buggy plugin can't crash the runtime.
        let resolver = KeybindingResolver::new(&empty_config());
        let outcome = resolver.register(
            "F3",
            ResolvedBinding::builtin(Action::Quit),
            ConflictPolicy::Override,
        );
        assert_eq!(outcome, RegisterOutcome::Invalid);
    }

    #[test]
    fn key_for_action_excludes_plugin_command_sentinel() {
        let resolver = KeybindingResolver::new(&empty_config());
        let _ = resolver.register(
            "Alt+Shift+K",
            ResolvedBinding::plugin("a.pairee", "entry"),
            ConflictPolicy::Override,
        );
        assert!(resolver.key_for_action(Action::PluginCommand).is_none());
        // A normal Builtin action still has a key.
        assert!(resolver.key_for_action(Action::Quit).is_some());
    }

    #[test]
    fn plugin_bindings_returns_only_plugin_sources() {
        let resolver = KeybindingResolver::new(&empty_config());
        let _ = resolver.register(
            "Alt+Shift+K",
            ResolvedBinding::plugin("a.pairee", "entry"),
            ConflictPolicy::Override,
        );
        let _ = resolver.register(
            "Alt+Shift+Q",
            ResolvedBinding::lua("b.pairee", "entry"),
            ConflictPolicy::Override,
        );
        let pb = resolver.plugin_bindings();
        assert_eq!(pb.len(), 2);
        let names: Vec<_> = pb.iter().map(|(_, p, _)| p.clone()).collect();
        assert!(names.contains(&"a.pairee".to_string()));
        assert!(names.contains(&"b.pairee".to_string()));
    }

    #[test]
    fn key_event_to_string_basic() {
        let key_up = KeyEvent::new(KeyCode::Up, KeyModifiers::empty());
        assert_eq!(key_event_to_string(key_up), "Up");

        let key_ctrl_h = KeyEvent::new(KeyCode::Char('h'), KeyModifiers::CONTROL);
        assert_eq!(key_event_to_string(key_ctrl_h), "Ctrl+h");

        let key_shift_tab = KeyEvent::new(KeyCode::Tab, KeyModifiers::SHIFT);
        assert_eq!(key_event_to_string(key_shift_tab), "Shift+Tab");
    }

    #[test]
    fn key_event_to_string_alt_f() {
        let key_alt_f7 = KeyEvent::new(KeyCode::F(7), KeyModifiers::ALT);
        assert_eq!(key_event_to_string(key_alt_f7), "Alt+F7");

        let key_shift_f9 = KeyEvent::new(KeyCode::F(9), KeyModifiers::SHIFT);
        assert_eq!(key_event_to_string(key_shift_f9), "Shift+F9");
    }

    #[test]
    fn key_event_to_string_ctrl_alt() {
        let key_ctrl_alt_1 = KeyEvent::new(
            KeyCode::Char('1'),
            KeyModifiers::CONTROL | KeyModifiers::ALT,
        );
        assert_eq!(key_event_to_string(key_ctrl_alt_1), "Ctrl+Alt+1");
    }
}
