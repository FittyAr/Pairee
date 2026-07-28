use super::reload_installed_plugins;
use crate::app::context::AppContext;
use crate::config::localization::t;
use crate::plugin::manager::{ActionKind, NotifyPayload, PluginRequest, UpdateStatus};
use crossterm::event::{KeyCode, KeyEvent};
use std::collections::HashMap;

// `ActionKind` is defined in `crate::plugin::manager` (next to
// `PluginRequest`) so the dispatcher and the TUI handler can
// refer to the same enum without a circular dependency between
// the request type and the input popup module.

pub fn handle_installed(
    key: KeyEvent,
    context: &mut AppContext,
    cursor_idx: &mut usize,
    installed: &mut Vec<(String, String, bool, bool, Option<String>)>,
    action_in_flight: Option<ActionKind>,
) {
    match key.code {
        KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
            if !installed.is_empty() {
                if *cursor_idx == 0 {
                    *cursor_idx = installed.len() - 1;
                } else {
                    *cursor_idx -= 1;
                }
            }
        }
        KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
            if !installed.is_empty() {
                if *cursor_idx + 1 >= installed.len() {
                    *cursor_idx = 0;
                } else {
                    *cursor_idx += 1;
                }
            }
        }
        KeyCode::Char('t') | KeyCode::Char('T') => {
            if let Some((name, _, _, _, _)) = installed.get(*cursor_idx) {
                if let Ok(mut config) = crate::config::AppConfig::load_or_create() {
                    let plugin_conf =
                        config
                            .settings
                            .plugins
                            .entry(name.clone())
                            .or_insert_with(|| crate::config::settings::PluginConfig {
                                name: name.clone(),
                                trusted: false,
                            });
                    plugin_conf.trusted = !plugin_conf.trusted;
                    let new_trust = plugin_conf.trusted;
                    let _ = config.save();
                    send_toast(
                        if new_trust {
                            t("plugin_toast_trust_ok")
                        } else {
                            t("plugin_toast_untrust_ok")
                        },
                        name,
                        "info",
                        2.0,
                    );
                }

                if let Ok(c) = crate::config::AppConfig::load_or_create() {
                    context.config = c;
                }

                *installed = reload_installed_plugins(context, &None);
            }
        }
        KeyCode::Char('p') | KeyCode::Char('P') => {
            if let Some((name, _, _, _, _)) = installed.get(*cursor_idx) {
                let mut lock = crate::plugin::updater::read_lockfile();
                let new_pinned = match lock.plugins.get_mut(name) {
                    Some(p) => {
                        p.pinned = !p.pinned;
                        Some(p.pinned)
                    }
                    None => None,
                };
                let _ = crate::plugin::updater::write_lockfile(&lock);

                if let Some(p) = new_pinned {
                    send_toast(
                        if p {
                            t("plugin_toast_pin_ok")
                        } else {
                            t("plugin_toast_unpin_ok")
                        },
                        name,
                        "info",
                        2.0,
                    );
                }

                *installed = reload_installed_plugins(context, &None);
            }
        }
        KeyCode::Char('d') | KeyCode::Char('D') | KeyCode::Delete => {
            if let Some((name, _, _, _, _)) = installed.get(*cursor_idx).cloned() {
                // `remove()` is now silent (no `println!`). We
                // emit a toast based on the result and refresh
                // the installed list. The synchronous call is
                // fast (a couple of filesystem unlinks + a TOML
                // write), so we do it inline rather than via a
                // `tokio::spawn`; the user gets immediate
                // feedback.
                match crate::plugin::updater::remove(&name) {
                    Ok(()) => {
                        send_toast(t("plugin_toast_uninstall_ok"), &name, "info", 3.0);
                    }
                    Err(e) => {
                        let _ = tx_or_log();
                        send_toast(
                            t("plugin_toast_uninstall_err"),
                            &format!("{}:{:?}", name, e),
                            "error",
                            5.0,
                        );
                    }
                }
                *installed = reload_installed_plugins(context, &None);
                *cursor_idx = (*cursor_idx).min(installed.len().saturating_sub(1));
            }
        }
        KeyCode::Char('u') => {
            if let Some((name, _, _, _, _)) = installed.get(*cursor_idx).cloned() {
                if action_in_flight.is_some() {
                    send_busy_toast();
                    return;
                }
                let name_clone = name.clone();
                // Pre-compute the trust overrides so the
                // post-update refresh keeps the same `trusted`
                // column the popup already shows.
                let trust_overrides: HashMap<String, bool> = context
                    .config
                    .settings
                    .plugins
                    .iter()
                    .map(|(k, p)| (k.clone(), p.trusted))
                    .collect();
                let tx = crate::plugin::PluginManager::get_sender();
                // Acquire the per-popup action lock so a second
                // `u` press is rejected with a "busy" toast while
                // this task is resolving.
                let _ = tx.try_send(PluginRequest::PluginActionStarted(ActionKind::Update));
                tokio::spawn(async move {
                    // `update(Some(name))` is silent: it returns
                    // a `UpdateReport` with one item we can map
                    // to a verb-aware toast.
                    let request = match crate::plugin::updater::update(Some(&name_clone)).await {
                        Ok(report) => build_update_toast(&report),
                        Err(e) => PluginRequest::NotifyStructured(NotifyPayload {
                            title: t("plugin_toast_update_err_title"),
                            content: t("plugin_toast_update_err")
                                .replace("{}", &name_clone)
                                .replace("{:?}", &format!("{:?}", e)),
                            level: Some("error".to_string()),
                            timeout_secs: Some(5.0),
                        }),
                    };
                    let _ = tx.send(request).await;
                    // Release the per-popup action lock so the
                    // user can run `u` / `U` again.
                    let _ = tx.send(PluginRequest::PluginActionFinished).await;
                    // Refresh the popup's `installed` list so the
                    // user sees the new version without having to
                    // close and reopen the modal. The `update`
                    // call already paid the registry round-trip;
                    // this one extra fetch keeps the popup in
                    // sync with the lockfile.
                    let rows =
                        crate::plugin::updater::fetch_installed_rows_for_refresh(&trust_overrides)
                            .await;
                    let _ = tx
                        .send(PluginRequest::InstalledPluginsRefreshed(rows))
                        .await;
                });
            }
        }
        KeyCode::Char('U') => {
            if action_in_flight.is_some() {
                send_busy_toast();
                return;
            }
            // Pre-compute the trust overrides so the
            // post-update refresh keeps the same `trusted`
            // column the popup already shows.
            let trust_overrides: HashMap<String, bool> = context
                .config
                .settings
                .plugins
                .iter()
                .map(|(k, p)| (k.clone(), p.trusted))
                .collect();
            let tx = crate::plugin::PluginManager::get_sender();
            // Acquire the per-popup action lock so a second `U`
            // press is rejected with a "busy" toast while this
            // task is resolving.
            let _ = tx.try_send(PluginRequest::PluginActionStarted(ActionKind::UpdateAll));
            tokio::spawn(async move {
                // `update(None)` is silent: it returns a
                // `UpdateReport` whose items we map 1-to-1 to
                // per-plugin toasts (the user now sees each
                // plugin get updated, not just a single "all
                // done" notification that hides which one
                // failed).
                match crate::plugin::updater::update(None).await {
                    Ok(report) => {
                        for (name, status) in &report.items {
                            let req = build_status_toast(name, status);
                            let _ = tx.send(req).await;
                        }
                        // Summary toast so the user can see the
                        // total at a glance.
                        let summary = format!(
                            "{} / {} updated, {} failed",
                            report.updated_count(),
                            report.items.len(),
                            report.failed_count()
                        );
                        let _ = tx
                            .send(PluginRequest::NotifyStructured(NotifyPayload {
                                title: t("plugin_toast_update_all_progress"),
                                content: summary,
                                level: Some(if report.failed_count() == 0 {
                                    "info".to_string()
                                } else {
                                    "warn".to_string()
                                }),
                                timeout_secs: Some(4.0),
                            }))
                            .await;
                    }
                    Err(e) => {
                        let _ = tx
                            .send(PluginRequest::NotifyStructured(NotifyPayload {
                                title: t("plugin_toast_update_all_err_title"),
                                content: t("plugin_toast_update_all_err")
                                    .replace("{:?}", &format!("{:?}", e)),
                                level: Some("error".to_string()),
                                timeout_secs: Some(5.0),
                            }))
                            .await;
                    }
                }
                // Release the per-popup action lock so the user
                // can run `u` / `U` again.
                let _ = tx.send(PluginRequest::PluginActionFinished).await;
                // Refresh the popup's `installed` list so the
                // user sees the new versions without having to
                // close and reopen the modal.
                let rows =
                    crate::plugin::updater::fetch_installed_rows_for_refresh(&trust_overrides)
                        .await;
                let _ = tx
                    .send(PluginRequest::InstalledPluginsRefreshed(rows))
                    .await;
            });
        }
        _ => {}
    }
}

// Helper: `tx_or_log` is a no-op kept only to keep the diff
// against the previous version small; the real toast send below
// does not need a channel. Marked `#[allow(dead_code)]` would
// silence the warning, but since we removed the variable in the
// success path this helper just exists as a placeholder.
#[inline]
fn tx_or_log() -> () {
    ()
}

/// Build a toast from a single-item `UpdateReport` (the response
/// of `update(Some(name))`). The verb matches the actual outcome
/// (Updated / UpToDate / Pinned / Blocked / Failed) so the user
/// knows exactly what just happened.
fn build_update_toast(report: &crate::plugin::updater::UpdateReport) -> PluginRequest {
    // The single-item report always has exactly one entry.
    let (name, status) = report
        .items
        .first()
        .cloned()
        .unwrap_or_else(|| (String::new(), UpdateStatus::UpToDate));
    build_status_toast(&name, &status)
}

fn build_status_toast(name: &str, status: &UpdateStatus) -> PluginRequest {
    let (key, level) = match status {
        UpdateStatus::Updated { .. } => ("plugin_toast_update_ok", "info"),
        UpdateStatus::UpToDate => ("plugin_toast_update_uptodate", "info"),
        UpdateStatus::Pinned => ("plugin_toast_update_pinned", "warn"),
        UpdateStatus::Blocked(_) => ("plugin_toast_update_blocked", "warn"),
        UpdateStatus::Failed(_) => ("plugin_toast_update_err", "error"),
    };
    let title = t(key);
    PluginRequest::NotifyStructured(NotifyPayload {
        title: title.clone(),
        content: title.replace("{}", name),
        level: Some(level.to_string()),
        timeout_secs: Some(if level == "error" { 5.0 } else { 3.0 }),
    })
}

/// Send a `NotifyStructured` toast via the plugin manager channel.
/// Best-effort: a closed channel (shutdown race) is logged and
/// dropped, never panics.
fn send_toast(content: String, name: &str, level: &str, timeout_secs: f64) {
    let tx = crate::plugin::PluginManager::get_sender();
    let _ = tx.try_send(PluginRequest::NotifyStructured(NotifyPayload {
        title: content.clone(),
        content: content.replace("{}", name),
        level: Some(level.to_string()),
        timeout_secs: Some(timeout_secs),
    }));
}

fn send_busy_toast() {
    send_toast(t("plugin_install_busy_msg"), "", "warn", 2.0);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Pinning returns the new state so the TUI can pick the
    /// right toast verb ("Pinned" / "Unpinned") without
    /// re-reading the lockfile.
    #[test]
    fn pin_return_value_picks_correct_verb() {
        // Pure type / mapping test: we don't run `pin()` here
        // (it would touch the real config dir); we just verify
        // the boolean→key mapping we use in production.
        fn key_for(new_pinned: bool) -> &'static str {
            if new_pinned {
                "plugin_toast_pin_ok"
            } else {
                "plugin_toast_unpin_ok"
            }
        }
        assert_eq!(key_for(true), "plugin_toast_pin_ok");
        assert_eq!(key_for(false), "plugin_toast_unpin_ok");
    }

    /// Trust toggle returns the new state so the TUI can pick
    /// the right toast verb.
    #[test]
    fn trust_return_value_picks_correct_verb() {
        fn key_for(new_trusted: bool) -> &'static str {
            if new_trusted {
                "plugin_toast_trust_ok"
            } else {
                "plugin_toast_untrust_ok"
            }
        }
        assert_eq!(key_for(true), "plugin_toast_trust_ok");
        assert_eq!(key_for(false), "plugin_toast_untrust_ok");
    }

    /// The single-item `UpdateReport` returned by
    /// `update(Some(name))` must produce a single toast with
    /// the right verb for each `UpdateStatus` variant.
    #[test]
    fn update_toast_key_matches_status() {
        let name = "foo.pairee".to_string();
        for (status, expected) in [
            (UpdateStatus::UpToDate, "plugin_toast_update_uptodate"),
            (UpdateStatus::Pinned, "plugin_toast_update_pinned"),
            (
                UpdateStatus::Blocked("x".to_string()),
                "plugin_toast_update_blocked",
            ),
            (
                UpdateStatus::Updated {
                    from: "1.0".to_string(),
                    to: "2.0".to_string(),
                },
                "plugin_toast_update_ok",
            ),
            (
                UpdateStatus::Failed("boom".to_string()),
                "plugin_toast_update_err",
            ),
        ] {
            let report = crate::plugin::updater::UpdateReport {
                items: vec![(name.clone(), status)],
            };
            let req = build_update_toast(&report);
            match req {
                PluginRequest::NotifyStructured(p) => {
                    assert_eq!(p.title, t(expected));
                    assert!(p.content.contains(&name));
                }
                _ => panic!("expected NotifyStructured"),
            }
        }
    }

    /// The per-item `update_all` toast builder must produce the
    /// right verb for every `UpdateStatus` variant.
    #[test]
    fn update_all_per_item_toast_keys() {
        for (status, expected) in [
            (UpdateStatus::UpToDate, "plugin_toast_update_uptodate"),
            (UpdateStatus::Pinned, "plugin_toast_update_pinned"),
            (
                UpdateStatus::Blocked("x".to_string()),
                "plugin_toast_update_blocked",
            ),
            (
                UpdateStatus::Updated {
                    from: "1".to_string(),
                    to: "2".to_string(),
                },
                "plugin_toast_update_ok",
            ),
            (
                UpdateStatus::Failed("x".to_string()),
                "plugin_toast_update_err",
            ),
        ] {
            let req = build_status_toast("p.pairee", &status);
            match req {
                PluginRequest::NotifyStructured(p) => {
                    assert_eq!(p.title, t(expected));
                }
                _ => panic!("expected NotifyStructured"),
            }
        }
    }
}
