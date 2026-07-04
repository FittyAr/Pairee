use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType};
use crate::fs::FileEntry;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::OnceLock;
use tokio::sync::{Mutex, mpsc, oneshot};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppStateSnapshot {
    pub active_panel: String,
    pub left_cwd: String,
    pub right_cwd: String,
    pub hovered_file: Option<FileEntrySnapshot>,
    pub selected_files: Vec<FileEntrySnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntrySnapshot {
    pub name: String,
    pub url: String,
    pub path: String,
    pub size: u64,
    pub is_dir: bool,
    pub is_symlink: bool,
}

impl FileEntrySnapshot {
    pub fn from_file_entry(entry: &FileEntry) -> Self {
        let path_str = entry.path.to_string_lossy().to_string();
        Self {
            name: entry.name.clone(),
            url: path_str.clone(),
            path: path_str,
            size: entry.size,
            is_dir: entry.is_dir,
            is_symlink: entry.is_symlink,
        }
    }
}

pub enum PluginRequest {
    GetStateSnapshot(oneshot::Sender<AppStateSnapshot>),
    Notify {
        title: String,
        msg: String,
        level: String,
    },
    Cd {
        path: String,
    },
    SetFocus {
        side: String,
    },
    Confirm {
        title: String,
        msg: String,
        reply_tx: oneshot::Sender<bool>,
    },
    Input {
        title: String,
        default: String,
        reply_tx: oneshot::Sender<String>,
    },
    SpawnCopyTask {
        from: PathBuf,
        to: PathBuf,
    },
    UpdatePluginWidget {
        path: PathBuf,
        widget: crate::app::state::types::PluginWidget,
    },
    /// Result of an asynchronous load of the installed-plugins list
    /// (triggered when opening the Plugin Manager). The receiver is the
    /// `(name, version, pinned, trusted, update_available)` tuple used by
    /// the `PluginMenu` popup's `installed` field.
    PluginMenuLoaded {
        installed: Vec<(String, String, bool, bool, Option<String>)>,
        registry: Vec<(String, String, String, String)>,
    },
    /// Result of an asynchronous scan of the dev plugins folder (and the two
    /// panel paths) for Option 0 "Select active development plugin".
    DevPluginScan {
        options: Vec<(String, String)>,
    },
}

static PLUGIN_REQ_TX: OnceLock<mpsc::Sender<PluginRequest>> = OnceLock::new();
static PLUGIN_REQ_RX: OnceLock<Mutex<mpsc::Receiver<PluginRequest>>> = OnceLock::new();

pub struct PluginManager;

impl PluginManager {
    pub fn init() {
        let (tx, rx) = mpsc::channel(100);
        let _ = PLUGIN_REQ_TX.set(tx);
        let _ = PLUGIN_REQ_RX.set(Mutex::new(rx));
        log::info!("PluginManager initialized request channels.");
    }

    pub fn get_sender() -> mpsc::Sender<PluginRequest> {
        PLUGIN_REQ_TX
            .get()
            .cloned()
            .expect("PluginManager channels not initialized")
    }

    pub async fn load_all_plugins(context: &AppContext) {
        let plugins_dir = crate::config::paths::get_config_dir().join("plugins");
        if !plugins_dir.exists() {
            let _ = std::fs::create_dir_all(&plugins_dir);
        }

        // Search directory for plugins
        if let Ok(entries) = std::fs::read_dir(&plugins_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let folder_name = path.file_name().unwrap().to_string_lossy().to_string();
                    if !folder_name.ends_with(".pairee") {
                        continue;
                    }
                    let name = folder_name.strip_suffix(".pairee").unwrap().to_string();
                    let enabled = context
                        .config
                        .settings
                        .plugins
                        .get(&name)
                        .map(|c| c.name == name)
                        .unwrap_or(true); // Enabled by default if not set otherwise

                    let trusted = context
                        .config
                        .settings
                        .plugins
                        .get(&name)
                        .map(|c| c.trusted)
                        .unwrap_or(false);

                    if enabled {
                        let tx = Self::get_sender();
                        let name_clone = name.clone();
                        let path_clone = path.clone();
                        tokio::spawn(async move {
                            log::info!("Loading plugin {} from {:?}", name_clone, path_clone);
                            if let Err(e) = crate::plugin::loader::load_plugin(
                                &name_clone,
                                &path_clone,
                                trusted,
                                tx,
                            )
                            .await
                            {
                                log::error!("Failed to load plugin {}: {:?}", name_clone, e);
                            }
                        });
                    }
                }
            }
        }
    }
}

/// Processes plugin requests in the main application loop.
pub fn process_plugin_requests(state: &mut AppState, context: &AppContext) {
    if let Some(rx_mutex) = PLUGIN_REQ_RX.get() {
        if let Ok(mut rx) = rx_mutex.try_lock() {
            while let Ok(req) = rx.try_recv() {
                match req {
                    PluginRequest::GetStateSnapshot(reply_tx) => {
                        let active = state.get_active_panel();
                        let hovered = active
                            .entries
                            .get(active.cursor_index)
                            .map(FileEntrySnapshot::from_file_entry);
                        let selected = active
                            .entries
                            .iter()
                            .filter(|e| active.selection_order.contains(&e.path))
                            .map(FileEntrySnapshot::from_file_entry)
                            .collect();

                        let snapshot = AppStateSnapshot {
                            active_panel: format!("{:?}", state.active_panel).to_lowercase(),
                            left_cwd: state.left_panel.current_path.to_string_lossy().to_string(),
                            right_cwd: state.right_panel.current_path.to_string_lossy().to_string(),
                            hovered_file: hovered,
                            selected_files: selected,
                        };
                        let _ = reply_tx.send(snapshot);
                    }
                    PluginRequest::Notify { title, msg, level } => {
                        state.active_popup = Some(PopupType::Info(format!("{}: {}", title, msg)));
                        log::info!("Plugin notify [{}]: {} - {}", level, title, msg);
                    }
                    PluginRequest::Cd { path } => {
                        let p = PathBuf::from(path);
                        state.get_active_panel_mut().current_path = p;
                        state.refresh_both_panels(context.config.settings.show_hidden);
                    }
                    PluginRequest::SetFocus { side } => {
                        if side == "left" {
                            state.active_panel = crate::app::state::ActivePanel::Left;
                        } else if side == "right" {
                            state.active_panel = crate::app::state::ActivePanel::Right;
                        }
                    }
                    PluginRequest::Confirm {
                        title,
                        msg,
                        reply_tx,
                    } => {
                        log::info!("Plugin confirm dialog requested: {} - {}", title, msg);
                        let _ = reply_tx.send(true);
                    }
                    PluginRequest::Input {
                        title,
                        default,
                        reply_tx,
                    } => {
                        log::info!("Plugin input dialog requested: {} - {}", title, default);
                        let _ = reply_tx.send(default);
                    }
                    PluginRequest::SpawnCopyTask { from, to } => {
                        log::info!("Plugin requesting copy from {:?} to {:?}", from, to);
                        let rx = crate::fs::spawn_copy_task(
                            vec![from.clone()],
                            to.clone(),
                            context.config.settings.clone(),
                        );
                        state.active_bg_op = Some(crate::app::state::BackgroundOpContext::Copy {
                            sources: vec![from],
                            dest: to,
                        });
                        state.progress_rx = Some(rx);
                        state.active_popup = Some(PopupType::CopyProgress {
                            is_move: false,
                            current_file: "Initializing...".to_string(),
                            files_copied: 0,
                            total_files: 0,
                            bytes_copied: 0,
                            total_bytes: 0,
                        });
                    }
                    PluginRequest::UpdatePluginWidget { path, widget } => {
                        if let Some(PopupType::QuickViewPanel {
                            path: ref cur_path,
                            ref mut plugin_widget,
                            ..
                        }) = state.active_popup
                        {
                            if cur_path == &path {
                                *plugin_widget = Some(widget);
                            }
                        }
                    }
                    PluginRequest::PluginMenuLoaded { installed, registry } => {
                        if let Some(PopupType::PluginMenu {
                            installed: ref mut existing,
                            all_registry: ref mut existing_all,
                            registry: ref mut existing_registry,
                            installed_loading: ref mut loading,
                            installed_loading_status: ref mut loading_status,
                            ..
                        }) = state.active_popup
                        {
                            *existing = installed;
                            // all_registry stays as the full list for filtering
                            *existing_all = registry.clone();
                            // registry shows all entries until the user narrows it
                            *existing_registry = registry;
                            *loading = false;
                            *loading_status = String::new();
                        }
                    }
                    PluginRequest::DevPluginScan { options } => {
                        // Convert the scan into an open SelectDevPlugin popup.
                        let previous_popup = state
                            .active_popup
                            .clone()
                            .map(Box::new)
                            .unwrap_or_else(|| {
                                Box::new(PopupType::Info(String::new()))
                            });
                        state.active_popup = Some(PopupType::SelectDevPlugin {
                            options,
                            cursor_idx: 0,
                            previous_popup,
                        });
                    }
                }
            }
        }
    }
}
