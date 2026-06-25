use std::path::PathBuf;
use std::collections::BTreeMap;

/// Returns a list of default bookmarks/shortcuts.
pub fn get_hotlist_bookmarks() -> Vec<(String, PathBuf)> {
    let mut bookmarks = Vec::new();
    if let Some(path) = directories::UserDirs::new().map(|u| u.home_dir().to_path_buf()) {
        bookmarks.push(("Home Directory".to_string(), path));
    }
    if let Some(path) =
        directories::UserDirs::new().and_then(|u| u.desktop_dir().map(|d| d.to_path_buf()))
    {
        bookmarks.push(("Desktop".to_string(), path));
    }
    if let Some(path) =
        directories::UserDirs::new().and_then(|u| u.document_dir().map(|d| d.to_path_buf()))
    {
        bookmarks.push(("Documents".to_string(), path));
    }
    if let Some(path) =
        directories::UserDirs::new().and_then(|u| u.download_dir().map(|d| d.to_path_buf()))
    {
        bookmarks.push(("Downloads".to_string(), path));
    }
    bookmarks.push((
        "System Root".to_string(),
        PathBuf::from(if cfg!(target_os = "windows") {
            "C:\\"
        } else {
            "/"
        }),
    ));
    bookmarks
}

pub fn load_user_menu_commands() -> BTreeMap<String, String> {
    let path = crate::config::paths::get_config_dir().join("usermenu.toml");
    let mut commands = BTreeMap::new();
    if let Ok(content) = std::fs::read_to_string(&path) {
        if let Ok(toml_val) = toml::from_str::<toml::Value>(&content) {
            if let Some(cmds) = toml_val.get("commands").and_then(|v| v.as_table()) {
                for (k, v) in cmds {
                    if let Some(cmd_str) = v.as_str() {
                        commands.insert(k.clone(), cmd_str.to_string());
                    }
                }
            }
        }
    }
    commands
}
