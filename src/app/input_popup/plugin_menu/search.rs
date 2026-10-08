use super::toast::{Toast, spawn_with_toast};
use crate::app::list_nav::{ListKey, ListKeys, list_key};
use crate::config::localization::t;
use crossterm::event::{KeyCode, KeyEvent};

pub fn handle_search(
    key: KeyEvent,
    cursor_idx: &mut usize,
    registry: &mut Vec<(String, String, String, String)>,
    all_registry: &[(String, String, String, String)],
    search_query: &mut String,
    editing_query: &mut bool,
) {
    // Navigation always works, also while typing the query.
    if list_key(ListKeys::FULL, key.code, cursor_idx, registry.len()) == ListKey::Moved {
        return;
    }
    match key.code {
        // ── Install selected plugin (only outside edit mode) ─────────────────
        KeyCode::Char('i') | KeyCode::Char('I') if !*editing_query => {
            if let Some((name, _, _, _)) = registry.get(*cursor_idx) {
                let name = name.clone();
                let ok = Toast {
                    title: t("plugin_toast_install_title"),
                    msg: t("plugin_toast_install_ok").replace("{}", &name),
                };
                let err = Toast {
                    title: t("plugin_toast_install_err_title"),
                    msg: t("plugin_toast_install_err").replace("{}", &name),
                };
                spawn_with_toast(
                    async move { crate::plugin::updater::install(&name, None).await },
                    ok,
                    err,
                );
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
