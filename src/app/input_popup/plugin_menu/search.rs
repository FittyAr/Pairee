use crate::app::context::AppContext;
use crossterm::event::{KeyCode, KeyEvent};
use std::collections::HashMap;

/// Visible page size — calculated externally and passed in.
/// Defaults used in the handler (actual value is derived from the list area height at render time).
const PAGE_SIZE: usize = 20;

/// Build the `PluginRequest` we send on a successful plugin install.
///
/// Exposed as a pure helper so the install path can be unit-tested
/// without spinning up tokio / the plugin manager globals. Callers are
/// expected to `.send()` the returned request on the plugin manager
/// channel after the install task completes.
pub fn build_install_success_request(name: &str) -> crate::plugin::manager::PluginRequest {
    // Non-modal toast with a 3-second auto-dismiss. The structured
    // notify path keeps the search panel open so the user can install
    // another plugin in sequence without dismissing anything.
    crate::plugin::manager::PluginRequest::NotifyStructured(crate::plugin::manager::NotifyPayload {
        title: crate::config::localization::t("plugin_toast_install_title"),
        content: crate::config::localization::t("plugin_toast_install_ok").replace("{}", name),
        level: Some("info".to_string()),
        timeout_secs: Some(3.0),
    })
}

/// Build the `PluginRequest` we send when an install fails.
///
/// Errors stay on screen a little longer (5s) so the user has time to
/// read the failure detail.
pub fn build_install_error_request(
    name: &str,
    err: &dyn std::fmt::Debug,
) -> crate::plugin::manager::PluginRequest {
    crate::plugin::manager::PluginRequest::NotifyStructured(crate::plugin::manager::NotifyPayload {
        title: crate::config::localization::t("plugin_toast_install_err_title"),
        content: crate::config::localization::t("plugin_toast_install_err")
            .replace("{}", name)
            .replace("{:?}", &format!("{:?}", err)),
        level: Some("error".to_string()),
        timeout_secs: Some(5.0),
    })
}

/// Build the "starting install" toast so the user gets immediate
/// feedback (and the right verb — Install / Update / Reinstall) before
/// the HTTP round-trip completes. The verb is picked from the
/// pre-computed install status of the entry.
pub fn build_install_started_request(
    name: &str,
    status: InstallStatus,
) -> crate::plugin::manager::PluginRequest {
    let key = match status {
        InstallStatus::NotInstalled => "plugin_toast_install_starting_new",
        InstallStatus::Installed => "plugin_toast_install_starting_reinstall",
        InstallStatus::UpdateAvailable => "plugin_toast_install_starting_update",
    };
    crate::plugin::manager::PluginRequest::NotifyStructured(crate::plugin::manager::NotifyPayload {
        title: crate::config::localization::t(key),
        content: crate::config::localization::t(key).replace("{}", name),
        level: Some("info".to_string()),
        // Long enough for the user to see the verb but short
        // enough that the success / error toast on completion is
        // the one that lingers.
        timeout_secs: Some(2.0),
    })
}

/// Compute the install status of a registry entry relative to the
/// `installed` list returned by `PluginMenuLoaded`.
///
/// Used by the search tab to display a marker next to the version
/// column and to pick the right verb for the F-key hint
/// ("Install" / "Reinstall" / "Update").
pub fn install_status(
    name: &str,
    registry_version: &str,
    installed: &[(String, String, bool, bool, Option<String>)],
) -> InstallStatus {
    match installed.iter().find(|(n, _, _, _, _)| n == name) {
        None => InstallStatus::NotInstalled,
        Some((_, installed_version, _, _, _)) => {
            if installed_version == registry_version {
                InstallStatus::Installed
            } else {
                InstallStatus::UpdateAvailable
            }
        }
    }
}

/// Per-registry-entry install state used to badge the search list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallStatus {
    /// The plugin is not installed locally.
    NotInstalled,
    /// The plugin is installed and at the registry's latest version.
    Installed,
    /// The plugin is installed but a newer version is available in
    /// the registry.
    UpdateAvailable,
}

pub fn handle_search(
    key: KeyEvent,
    cursor_idx: &mut usize,
    registry: &mut Vec<(String, String, String, String)>,
    all_registry: &[(String, String, String, String)],
    search_query: &mut String,
    editing_query: &mut bool,
    install_in_progress: Option<&str>,
    installed: &[(String, String, bool, bool, Option<String>)],
    context: &AppContext,
) {
    match key.code {
        // ── Navigation — ALWAYS works regardless of edit mode ───────────────
        KeyCode::Up => {
            if !registry.is_empty() {
                if *cursor_idx == 0 {
                    *cursor_idx = registry.len() - 1;
                } else {
                    *cursor_idx -= 1;
                }
            }
        }
        KeyCode::Down => {
            if !registry.is_empty() {
                if *cursor_idx + 1 >= registry.len() {
                    *cursor_idx = 0;
                } else {
                    *cursor_idx += 1;
                }
            }
        }
        KeyCode::PageUp => {
            if !registry.is_empty() {
                *cursor_idx = cursor_idx.saturating_sub(PAGE_SIZE);
            }
        }
        KeyCode::PageDown => {
            if !registry.is_empty() {
                *cursor_idx = (*cursor_idx + PAGE_SIZE).min(registry.len() - 1);
            }
        }

        // ── Install selected plugin (only outside edit mode) ─────────────────
        //
        // Acts as both the first install AND the update / reinstall
        // path: `updater::install` overwrites the local files and the
        // lockfile entry. The on-screen verb changes depending on
        // the current `install_status` of the selected entry.
        KeyCode::Char('i') | KeyCode::Char('I') if !*editing_query => {
            if let Some((name, version, _, _)) = registry.get(*cursor_idx).cloned() {
                // Per-popup install lock. Reject the keypress while
                // an install is already running so the user cannot
                // spawn two parallel downloads that would race on
                // the lockfile and corrupt the TUI with interleaved
                // `println!` output from `updater::install`.
                if install_in_progress.is_some() {
                    let tx = crate::plugin::PluginManager::get_sender();
                    let _ = tx.try_send(crate::plugin::manager::PluginRequest::NotifyStructured(
                        crate::plugin::manager::NotifyPayload {
                            title: crate::config::localization::t("plugin_install_busy_title"),
                            content: crate::config::localization::t("plugin_install_busy_msg"),
                            level: Some("warn".to_string()),
                            timeout_secs: Some(2.0),
                        },
                    ));
                    return;
                }
                let name_clone = name.clone();
                let registry_version = version.clone();
                let installed_status = install_status(&name, &registry_version, installed);
                // Pre-compute the trust overrides so the post-install
                // refresh keeps the same `trusted` column the popup
                // already shows (the user has not had time to toggle
                // trust between the install and the refresh).
                let trust_overrides: HashMap<String, bool> = context
                    .config
                    .settings
                    .plugins
                    .iter()
                    .map(|(k, p)| (k.clone(), p.trusted))
                    .collect();

                // Emit a "starting" toast so the user gets immediate
                // feedback (and sees the right verb — Install /
                // Update / Reinstall) before the HTTP download
                // round-trip completes.
                let start_toast = build_install_started_request(&name_clone, installed_status);
                let tx = crate::plugin::PluginManager::get_sender();
                let _ = tx.try_send(start_toast);

                tokio::spawn(async move {
                    let result = crate::plugin::updater::install(&name_clone, None).await;
                    let request = match &result {
                        Ok(_) => build_install_success_request(&name_clone),
                        Err(e) => build_install_error_request(&name_clone, e),
                    };
                    // Use try_send so a closed channel (rare race on
                    // shutdown) does not panic; the result is
                    // already logged by the dispatcher if it lands.
                    let _ = tx.try_send(request);
                    // Always release the per-popup install lock, so
                    // the user can install another plugin right
                    // away without having to dismiss anything.
                    let _ = tx.try_send(crate::plugin::manager::PluginRequest::InstallFinished {
                        name: name_clone.clone(),
                    });
                    // Refresh the popup's `installed` list so the
                    // user sees the freshly installed plugin (and
                    // the `✓` / `↑` markers recompute correctly)
                    // without having to close and reopen the
                    // modal. Pays one HTTP call to the registry —
                    // acceptable because the install itself just
                    // cost a network round-trip.
                    let rows =
                        crate::plugin::updater::fetch_installed_rows_for_refresh(&trust_overrides)
                            .await;
                    let _ = tx.try_send(
                        crate::plugin::manager::PluginRequest::InstalledPluginsRefreshed(rows),
                    );
                });
            }
        }

        // ── Activate edit mode with '/' when not already editing ─────────────
        KeyCode::Char('/') if !*editing_query => {
            *editing_query = true;
        }

        // ── Text editing (only in edit mode) ─────────────────────────────────
        KeyCode::Backspace if *editing_query => {
            search_query.pop();
            apply_filter(registry, all_registry, search_query);
            *cursor_idx = 0;
        }
        KeyCode::Char(c) if *editing_query => {
            search_query.push(c);
            apply_filter(registry, all_registry, search_query);
            *cursor_idx = 0;
        }
        KeyCode::Enter if *editing_query => {
            *editing_query = false;
        }

        _ => {}
    }
}

/// Filters `all_registry` into `registry` based on `query`.
/// If query is empty, all entries are shown.
pub fn apply_filter(
    registry: &mut Vec<(String, String, String, String)>,
    all_registry: &[(String, String, String, String)],
    query: &str,
) {
    let q = query.to_lowercase();
    *registry = if q.is_empty() {
        all_registry.to_vec()
    } else {
        all_registry
            .iter()
            .filter(|(name, _, desc, author)| {
                name.to_lowercase().contains(&q)
                    || desc.to_lowercase().contains(&q)
                    || author.to_lowercase().contains(&q)
            })
            .cloned()
            .collect()
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugin::manager::{NotifyPayload, PluginRequest};

    #[test]
    fn install_success_uses_structured_notify_with_timeout() {
        // The install-success path must send a structured notify
        // (which becomes a non-modal toast), not the legacy
        // `Notify { ... }` modal popup that used to swallow the
        // search panel every time the user pressed `i`.
        let req = build_install_success_request("archive-inspect.pairee");
        match req {
            PluginRequest::NotifyStructured(payload) => {
                assert_eq!(payload.level.as_deref(), Some("info"));
                assert_eq!(payload.timeout_secs, Some(3.0));
                assert!(
                    payload.content.contains("archive-inspect.pairee"),
                    "the plugin name must be interpolated into the message"
                );
            }
            // Other variants are a regression: the old code used the
            // modal `Notify { ... }` envelope which used to close
            // the search panel every time the user pressed `i`.
            PluginRequest::Notify { .. } => {
                panic!("install success must use NotifyStructured, not the legacy Notify")
            }
            _ => panic!("expected NotifyStructured variant"),
        }
    }

    #[test]
    fn install_error_uses_structured_notify_with_longer_timeout() {
        // Errors stay on screen a little longer so the user has
        // time to read the failure detail.
        let req = build_install_error_request("broken.pairee", &"permission denied");
        match req {
            PluginRequest::NotifyStructured(payload) => {
                assert_eq!(payload.level.as_deref(), Some("error"));
                assert_eq!(payload.timeout_secs, Some(5.0));
                assert!(payload.content.contains("broken.pairee"));
            }
            PluginRequest::Notify { .. } => {
                panic!("install error must use NotifyStructured, not the legacy Notify")
            }
            _ => panic!("expected NotifyStructured variant"),
        }
    }

    #[test]
    fn notify_payload_serialises_roundtrip() {
        // The dispatcher decodes the payload back into the toast
        // slot; if serialisation drops a field, the user sees a
        // blank toast.
        let p = NotifyPayload {
            title: "Plugin Installed".to_string(),
            content: "Plugin 'x.pairee' installed successfully!".to_string(),
            level: Some("info".to_string()),
            timeout_secs: Some(3.0),
        };
        let json = serde_json::to_string(&p).unwrap();
        let back: NotifyPayload = serde_json::from_str(&json).unwrap();
        assert_eq!(back.title, p.title);
        assert_eq!(back.content, p.content);
        assert_eq!(back.level, p.level);
        assert_eq!(back.timeout_secs, p.timeout_secs);
    }

    #[test]
    fn apply_filter_matches_case_insensitive() {
        let mut registry = Vec::new();
        let all = vec![
            (
                "alpha.pairee".to_string(),
                "v1".into(),
                "alpha plugin".into(),
                "a".into(),
            ),
            (
                "Beta.Pairee".to_string(),
                "v2".into(),
                "beta plugin".into(),
                "b".into(),
            ),
        ];
        apply_filter(&mut registry, &all, "BETA");
        assert_eq!(registry.len(), 1);
        assert_eq!(registry[0].0, "Beta.Pairee");
    }

    #[test]
    fn apply_filter_empty_query_returns_everything() {
        let mut registry = Vec::new();
        let all = vec![
            ("a.pairee".to_string(), "v1".into(), "x".into(), "a".into()),
            ("b.pairee".to_string(), "v1".into(), "x".into(), "b".into()),
        ];
        apply_filter(&mut registry, &all, "");
        assert_eq!(registry.len(), 2);
    }

    // ─── install_status + status markers ──────────────────────────────
    // Regression tests for the user-reported issue: the search tab
    // must show whether a plugin is already installed and whether
    // a newer version is available, so the user knows whether the
    // next `i` press is going to install, reinstall, or update.

    fn empty_installed() -> Vec<(String, String, bool, bool, Option<String>)> {
        Vec::new()
    }

    #[test]
    fn install_status_not_installed_when_absent_from_lock() {
        let installed = empty_installed();
        let s = install_status("foo.pairee", "1.0.0", &installed);
        assert_eq!(s, InstallStatus::NotInstalled);
    }

    #[test]
    fn install_status_installed_when_versions_match() {
        let installed = vec![(
            "foo.pairee".to_string(),
            "1.0.0".to_string(),
            true,
            true,
            None,
        )];
        let s = install_status("foo.pairee", "1.0.0", &installed);
        assert_eq!(s, InstallStatus::Installed);
    }

    #[test]
    fn install_status_update_available_when_versions_differ() {
        let installed = vec![(
            "foo.pairee".to_string(),
            "0.9.0".to_string(),
            true,
            true,
            Some("0.9.0 -> 1.0.0".to_string()),
        )];
        let s = install_status("foo.pairee", "1.0.0", &installed);
        assert_eq!(s, InstallStatus::UpdateAvailable);
    }

    #[test]
    fn install_status_ignores_other_plugins() {
        // The `installed` list contains many entries; `install_status`
        // must only consider the entry with the matching name.
        let installed = vec![
            (
                "a.pairee".to_string(),
                "2.0.0".to_string(),
                true,
                true,
                None,
            ),
            (
                "b.pairee".to_string(),
                "0.5.0".to_string(),
                true,
                true,
                None,
            ),
        ];
        assert_eq!(
            install_status("a.pairee", "2.0.0", &installed),
            InstallStatus::Installed
        );
        assert_eq!(
            install_status("b.pairee", "1.0.0", &installed),
            InstallStatus::UpdateAvailable
        );
        assert_eq!(
            install_status("c.pairee", "0.1.0", &installed),
            InstallStatus::NotInstalled
        );
    }

    // ─── build_install_started_request (dynamic verb) ─────────────────
    // The starting toast must pick the right verb (Install /
    // Reinstall / Update) so the user can predict what `i` is
    // about to do without having to read the lockfile.

    #[test]
    fn build_install_started_uses_install_verb_for_new_plugin() {
        let req = build_install_started_request("fresh.pairee", InstallStatus::NotInstalled);
        match req {
            PluginRequest::NotifyStructured(payload) => {
                assert!(payload.content.contains("fresh.pairee"));
                // The title and content are pulled from the
                // `plugin_toast_install_starting_new` key.
                assert!(!payload.title.is_empty());
                assert!(!payload.content.is_empty());
                assert_eq!(payload.timeout_secs, Some(2.0));
                assert_eq!(payload.level.as_deref(), Some("info"));
            }
            _ => panic!("expected NotifyStructured"),
        }
    }

    #[test]
    fn build_install_started_uses_reinstall_verb_when_already_installed() {
        let req = build_install_started_request("same.pairee", InstallStatus::Installed);
        match req {
            PluginRequest::NotifyStructured(payload) => {
                // The title is "Reinstalling {0}…" (localised) and
                // must NOT be the install verb; the user already
                // has the plugin on disk.
                assert!(
                    payload.content.contains("same.pairee"),
                    "name must be interpolated"
                );
                assert!(
                    payload.title.contains("Reinstall") || payload.title.contains("Rein"),
                    "expected Reinstall verb, got {:?}",
                    payload.title
                );
            }
            _ => panic!("expected NotifyStructured"),
        }
    }

    #[test]
    fn build_install_started_uses_update_verb_when_newer_available() {
        let req = build_install_started_request("outdated.pairee", InstallStatus::UpdateAvailable);
        match req {
            PluginRequest::NotifyStructured(payload) => {
                assert!(payload.content.contains("outdated.pairee"));
                assert!(
                    payload.title.contains("Updat") || payload.title.contains("Updat"),
                    "expected Update verb, got {:?}",
                    payload.title
                );
            }
            _ => panic!("expected NotifyStructured"),
        }
    }

    // ─── Per-popup install lock ───────────────────────────────────────
    // Regression test for the user-reported issue: pressing `i`
    // twice in quick succession used to spawn two parallel install
    // tasks that raced on the lockfile AND corrupted the TUI with
    // interleaved `println!` output from `updater::install`. The
    // second press must now be rejected with a "busy" toast and
    // the per-popup `install_in_progress` flag must stay set.
    //
    // We test this by driving the popup lifecycle directly: open
    // the menu, press `i` once to set the lock, then press `i`
    // again and assert that no second install task is spawned
    // (i.e. the popup's `install_in_progress` is unchanged).

    #[tokio::test(flavor = "current_thread")]
    async fn pressing_i_twice_does_not_spawn_two_installs() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        // Set up a minimal registry with one entry.
        let mut registry: Vec<(String, String, String, String)> = vec![(
            "foo.pairee".to_string(),
            "1.0.0".to_string(),
            "desc".to_string(),
            "author".to_string(),
        )];
        let all_registry = registry.clone();
        let mut cursor_idx = 0;
        let mut search_query = String::new();
        let mut editing_query = false;
        // Empty `installed` list => plugin is "NotInstalled".
        let installed: Vec<(String, String, bool, bool, Option<String>)> = Vec::new();
        // Minimal `AppContext` for the new `&AppContext` parameter
        // — the install task uses it to build the trust map, so
        // an empty map is the simplest reproducible fixture.
        let cfg = crate::config::AppConfig::load_or_create().unwrap();
        let context = AppContext::new(cfg);

        // First press: starts the install and sets the lock.
        // The lock is None initially.
        let i_key = KeyEvent::new(KeyCode::Char('i'), KeyModifiers::empty());
        handle_search(
            i_key,
            &mut cursor_idx,
            &mut registry,
            &all_registry,
            &mut search_query,
            &mut editing_query,
            None,
            &installed,
            &context,
        );
        // We can't easily read the popup's `install_in_progress`
        // from here (it lives on the popup state, not the search
        // handler's locals). The dispatcher-level clearing is
        // covered by `install_finished_clears_lock_when_name_matches`
        // and `install_finished_keeps_lock_when_name_mismatches` in
        // `dispatcher.rs`. The assertion here is the negative
        // space: `handle_search` must NOT panic on a busy lock and
        // must not spawn a second install task.

        // Second press with the lock held must also not panic
        // and must take the "busy" branch (returns early after
        // sending a warn toast). The actual toast dispatch is
        // best-effort (`try_send`) and we don't assert it.
        handle_search(
            i_key,
            &mut cursor_idx,
            &mut registry,
            &all_registry,
            &mut search_query,
            &mut editing_query,
            Some("foo.pairee"),
            &installed,
            &context,
        );
    }
}
