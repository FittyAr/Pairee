use crate::config::localization::t;
use crossterm::event::{KeyCode, KeyEvent};

pub fn handle_search(
    key: KeyEvent,
    cursor_idx: &mut usize,
    registry: &mut Vec<(String, String, String, String)>,
    search_query: &mut String,
    editing_query: &mut bool,
) {
    if *editing_query {
        match key.code {
            KeyCode::Backspace => {
                search_query.pop();
            }
            KeyCode::Char(c) => {
                search_query.push(c);
            }
            KeyCode::Enter => {
                *editing_query = false;
                let index_res = tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current()
                        .block_on(crate::plugin::updater::fetch_index())
                });
                if let Ok(idx) = index_res {
                    registry.clear();
                    let q_lower = search_query.to_lowercase();
                    for (name, p) in &idx.plugins {
                        if name.to_lowercase().contains(&q_lower)
                            || p.description
                                .as_ref()
                                .map(|d| d.to_lowercase().contains(&q_lower))
                                .unwrap_or(false)
                        {
                            registry.push((
                                name.clone(),
                                p.version.clone(),
                                p.description.clone().unwrap_or_default(),
                                p.author.clone().unwrap_or_default(),
                            ));
                        }
                    }
                }
                *cursor_idx = 0;
            }
            _ => {}
        }
    } else {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                if !registry.is_empty() {
                    if *cursor_idx == 0 {
                        *cursor_idx = registry.len() - 1;
                    } else {
                        *cursor_idx -= 1;
                    }
                }
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                if !registry.is_empty() {
                    if *cursor_idx + 1 >= registry.len() {
                        *cursor_idx = 0;
                    } else {
                        *cursor_idx += 1;
                    }
                }
            }
            KeyCode::Char('/') => {
                *editing_query = true;
            }
            KeyCode::Char('i') | KeyCode::Char('I') => {
                if !registry.is_empty() {
                    if let Some((name, _, _, _)) = registry.get(*cursor_idx) {
                        let name_clone = name.clone();
                        let tx = crate::plugin::PluginManager::get_sender();
                        tokio::spawn(async move {
                            match crate::plugin::updater::install(&name_clone, None).await {
                                Ok(_) => {
                                    let _ = tx
                                        .send(crate::plugin::manager::PluginRequest::Notify {
                                            title: t("plugin_toast_install_title"),
                                            msg: t("plugin_toast_install_ok")
                                                .replace("{}", &name_clone),
                                            level: "info".to_string(),
                                        })
                                        .await;
                                }
                                Err(e) => {
                                    let _ = tx
                                        .send(crate::plugin::manager::PluginRequest::Notify {
                                            title: t("plugin_toast_install_err_title"),
                                            msg: t("plugin_toast_install_err")
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
            }
            _ => {}
        }
    }
}
