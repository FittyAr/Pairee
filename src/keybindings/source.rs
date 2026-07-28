//! Source attribution for resolved keybindings.
//!
//! Every key in the [`KeybindingResolver`](super::resolver::KeybindingResolver)
//! carries provenance: was the binding installed by the built-in
//! preset, the user's `keybindings.toml`, a plugin manifest, or a
//! runtime Lua call? Plugins and Lua can use this information to
//! (a) refuse to take a Builtin key, (b) report conflicts to the
//! user, and (c) render a more useful F-key bar.
//!
//! ## Priority
//!
//! When two sources register the same key, the one with the higher
//! [`priority`](BindingSource::priority) wins; the loser is logged
//! as a conflict and not stored. Priorities are fixed:
//!
//! | Source       | Priority | Notes                                  |
//! |--------------|----------|----------------------------------------|
//! | `User`       | 100      | `keybindings.toml` `[custom_bindings]` |
//! | `Lua`        | 80       | `pairee.keybindings.bind()`            |
//! | `Plugin`     | 60       | `manifest.toml` `[keybindings]`        |
//! | `Builtin`    | 40       | the active preset                      |
//!
//! `User` beats `Lua` beats `Plugin` beats `Builtin` — i.e. by
//! default the preset is the floor; the user can always re-claim
//! a key with `keybindings.toml`, and plugins can take over keys
//! that the preset leaves unbound.
//!
//! Plugins that want to override a Builtin key must opt in via
//! [`ConflictPolicy::Steal`](super::source::ConflictPolicy) and
//! be installed in a slot with the matching priority. Today the
//! resolver does not auto-promote Plugins above Builtin (that
//! would silently break users who have come to rely on a key);
//! a plugin that asks for a Builtin key with the default
//! `Fallback` policy is rejected with a `log::warn!` line so the
//! user has a chance to find it in the app log.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Provenance of a resolved keybinding.
///
/// Cloning is cheap (an enum + a small `String`); the resolver
/// stores one of these per registered key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum BindingSource {
    /// Loaded from the active built-in preset (`norton`, `neovim`, `vscode`).
    Builtin,
    /// User override in `keybindings.toml` `[custom_bindings]`.
    User,
    /// Declared by a plugin in its `manifest.toml` `[keybindings]`
    /// table. The carrying plugin is identified by name.
    Plugin { plugin: String },
    /// Registered at runtime from Lua via `pairee.keybindings.bind`.
    /// The carrying plugin is identified by name.
    Lua { plugin: String },
}

impl BindingSource {
    /// Higher priority wins when two sources compete for the same key.
    /// See the module-level table for the full ordering.
    pub const fn priority(&self) -> u8 {
        match self {
            BindingSource::User => 100,
            BindingSource::Lua { .. } => 80,
            BindingSource::Plugin { .. } => 60,
            BindingSource::Builtin => 40,
        }
    }

    /// Short, human-readable label for UI / log lines.
    pub fn label(&self) -> String {
        match self {
            BindingSource::Builtin => "builtin".to_string(),
            BindingSource::User => "user".to_string(),
            BindingSource::Plugin { plugin } => format!("plugin:{}", plugin),
            BindingSource::Lua { plugin } => format!("lua:{}", plugin),
        }
    }
}

impl fmt::Display for BindingSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.label())
    }
}

/// A binding as it lives in the resolver: the action it triggers
/// and where the binding came from.
///
/// The resolver never returns a bare `Action` anymore; it always
/// returns a `ResolvedBinding` so the dispatcher can decide
/// whether to route to the built-in action handler or to the
/// plugin runner based on [`source`](Self::source).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolvedBinding {
    /// The action the keypress triggers. For Plugin / Lua sources
    /// this is a sentinel [`Action::PluginCommand`](super::actions::Action::PluginCommand)
    /// and the plugin name + custom action name are carried on
    /// [`source`](Self::source) instead.
    pub action: super::actions::Action,
    /// Provenance of the binding.
    pub source: BindingSource,
    /// For `Plugin` / `Lua` sources, the action name the plugin
    /// asked for (e.g. `"entry"`, `"peek"`, `"seek"`). Empty
    /// for `Builtin` / `User` sources — they carry the action
    /// directly in [`action`](Self::action).
    pub plugin_action: String,
}

impl ResolvedBinding {
    /// Build a `ResolvedBinding` from a preset or user source.
    pub const fn builtin(action: super::actions::Action) -> Self {
        Self {
            action,
            source: BindingSource::Builtin,
            plugin_action: String::new(),
        }
    }

    /// Build a `ResolvedBinding` from a user source.
    #[cfg(test)]
    pub const fn user(action: super::actions::Action) -> Self {
        Self {
            action,
            source: BindingSource::User,
            plugin_action: String::new(),
        }
    }

    /// Build a `ResolvedBinding` that the dispatcher will route
    /// to a plugin's `entry()` callback.
    pub fn plugin(plugin: impl Into<String>, plugin_action: impl Into<String>) -> Self {
        Self {
            action: super::actions::Action::PluginCommand,
            source: BindingSource::Plugin {
                plugin: plugin.into(),
            },
            plugin_action: plugin_action.into(),
        }
    }

    /// Build a `ResolvedBinding` that the dispatcher will route
    /// to a Lua callback registered at runtime.
    pub fn lua(plugin: impl Into<String>, plugin_action: impl Into<String>) -> Self {
        Self {
            action: super::actions::Action::PluginCommand,
            source: BindingSource::Lua {
                plugin: plugin.into(),
            },
            plugin_action: plugin_action.into(),
        }
    }

    /// True when the binding came from a Builtin preset. The
    /// dispatcher uses this to decide between the built-in action
    /// handler and the plugin runner.
    #[cfg(test)]
    pub const fn is_builtin(&self) -> bool {
        matches!(self.source, BindingSource::Builtin)
    }

    /// True when the binding was installed by the user (custom_bindings).
    #[cfg(test)]
    pub const fn is_user(&self) -> bool {
        matches!(self.source, BindingSource::User)
    }

    /// True when the binding was installed by a plugin (manifest or Lua).
    #[cfg(test)]
    pub const fn is_plugin(&self) -> bool {
        matches!(
            self.source,
            BindingSource::Plugin { .. } | BindingSource::Lua { .. }
        )
    }
}

/// What to do when registering a key that is already taken.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictPolicy {
    /// Reject the new binding and keep the old one. The default
    /// for plugin manifest entries and for user / builtin
    /// registrations.
    Fallback,
    /// Replace the old binding with the new one. Only allowed when
    /// the new source has a higher priority than the old one.
    /// Used internally by the resolver to let the user always
    /// win over plugins and let plugins always win over the
    /// built-in preset (when they opt in via this policy).
    Override,
}

/// Outcome of [`KeybindingResolver::register`](super::resolver::KeybindingResolver::register).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegisterOutcome {
    /// The key was registered (or replaced) and is now bound.
    Bound,
    /// The key was already bound by a higher-priority source and
    /// the new one was rejected. The existing binding is unchanged.
    Conflict { with: BindingSource },
    /// The key string is malformed and cannot be parsed.
    Invalid,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keybindings::Action;

    #[test]
    fn priority_order_is_user_over_lua_over_plugin_over_builtin() {
        // Lock the priority table so a future edit does not flip
        // it silently and break every plugin that depends on the
        // "user can always re-claim" guarantee.
        let order = [
            BindingSource::Builtin.priority(),
            BindingSource::Plugin { plugin: "x".into() }.priority(),
            BindingSource::Lua { plugin: "x".into() }.priority(),
            BindingSource::User.priority(),
        ];
        let mut sorted = order;
        sorted.sort();
        assert_eq!(order, sorted, "priority order must be monotonic");
    }

    #[test]
    fn label_is_stable_and_human_readable() {
        assert_eq!(BindingSource::Builtin.label(), "builtin");
        assert_eq!(BindingSource::User.label(), "user");
        assert_eq!(
            BindingSource::Plugin {
                plugin: "archive-inspect.pairee".into()
            }
            .label(),
            "plugin:archive-inspect.pairee"
        );
        assert_eq!(
            BindingSource::Lua {
                plugin: "fzf.pairee".into()
            }
            .label(),
            "lua:fzf.pairee"
        );
    }

    #[test]
    fn builtin_and_user_factories_leave_plugin_action_empty() {
        let b = ResolvedBinding::builtin(Action::Quit);
        assert!(b.is_builtin());
        assert!(b.plugin_action.is_empty());
        assert_eq!(b.action, Action::Quit);

        let u = ResolvedBinding::user(Action::MoveUp);
        assert!(u.is_user());
        assert!(u.plugin_action.is_empty());
    }

    #[test]
    fn plugin_factory_uses_plugin_command_sentinel() {
        let p = ResolvedBinding::plugin("disk-usage.pairee", "entry");
        assert!(p.is_plugin());
        assert!(!p.is_builtin());
        assert!(!p.is_user());
        assert_eq!(p.action, Action::PluginCommand);
        assert_eq!(p.plugin_action, "entry");
        assert_eq!(
            p.source,
            BindingSource::Plugin {
                plugin: "disk-usage.pairee".into()
            }
        );
    }

    #[test]
    fn lua_factory_uses_plugin_command_sentinel() {
        let l = ResolvedBinding::lua("fzf.pairee", "entry");
        assert!(l.is_plugin());
        assert!(matches!(l.source, BindingSource::Lua { .. }));
        assert_eq!(l.action, Action::PluginCommand);
        assert_eq!(l.plugin_action, "entry");
    }
}
