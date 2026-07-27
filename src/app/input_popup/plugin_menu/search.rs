use crossterm::event::{KeyCode, KeyEvent};

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

pub fn handle_search(
    key: KeyEvent,
    cursor_idx: &mut usize,
    registry: &mut Vec<(String, String, String, String)>,
    all_registry: &[(String, String, String, String)],
    search_query: &mut String,
    editing_query: &mut bool,
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
        KeyCode::Char('i') | KeyCode::Char('I') if !*editing_query => {
            if let Some((name, _, _, _)) = registry.get(*cursor_idx) {
                let name_clone = name.clone();
                let tx = crate::plugin::PluginManager::get_sender();
                tokio::spawn(async move {
                    let result = crate::plugin::updater::install(&name_clone, None).await;
                    let request = match &result {
                        Ok(_) => build_install_success_request(&name_clone),
                        Err(e) => build_install_error_request(&name_clone, e),
                    };
                    let _ = tx.send(request).await;
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
}
