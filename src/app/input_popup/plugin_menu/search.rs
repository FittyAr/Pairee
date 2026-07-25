use crossterm::event::{KeyCode, KeyEvent};

/// Visible page size — calculated externally and passed in.
/// Defaults used in the handler (actual value is derived from the list area height at render time).
const PAGE_SIZE: usize = 20;

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
                    match crate::plugin::updater::install(&name_clone, None).await {
                        Ok(_) => {
                            let _ = tx
                                .send(crate::plugin::manager::PluginRequest::Notify {
                                    title: crate::config::localization::t(
                                        "plugin_toast_install_title",
                                    ),
                                    msg: crate::config::localization::t("plugin_toast_install_ok")
                                        .replace("{}", &name_clone),
                                    level: "info".to_string(),
                                })
                                .await;
                        }
                        Err(e) => {
                            let _ = tx
                                .send(crate::plugin::manager::PluginRequest::Notify {
                                    title: crate::config::localization::t(
                                        "plugin_toast_install_err_title",
                                    ),
                                    msg: crate::config::localization::t("plugin_toast_install_err")
                                        .replace("{}", &name_clone)
                                        .replace("{:?}", &format!("{:?}", e)),
                                    level: "error".to_string(),
                                })
                                .await;
                        }
                    }
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
