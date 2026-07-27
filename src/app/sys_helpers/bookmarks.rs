use std::collections::BTreeMap;
use std::path::PathBuf;

/// Returns a list of default bookmarks/shortcuts.
pub fn get_hotlist_bookmarks() -> Vec<(String, PathBuf)> {
    let mut bookmarks = Vec::new();
    // The previous code called `directories::UserDirs::new()`
    // four separate times to extract `home_dir`, `desktop_dir`,
    // `document_dir` and `download_dir`. Each call is a mini-
    // scan of env vars (and on some distros, a hit to
    // `passwd` / `useradd` shellouts), so the duplication was a
    // measurable startup cost. We now call it once and read
    // every field from the same instance.
    if let Some(user) = directories::UserDirs::new() {
        bookmarks.push(("Home Directory".to_string(), user.home_dir().to_path_buf()));
        if let Some(p) = user.desktop_dir().map(|d| d.to_path_buf()) {
            bookmarks.push(("Desktop".to_string(), p));
        }
        if let Some(p) = user.document_dir().map(|d| d.to_path_buf()) {
            bookmarks.push(("Documents".to_string(), p));
        }
        if let Some(p) = user.download_dir().map(|d| d.to_path_buf()) {
            bookmarks.push(("Downloads".to_string(), p));
        }
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
