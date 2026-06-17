pub fn command_exists(cmd: &str) -> bool {
    let cmd_name = match cmd.split_whitespace().next() {
        Some(name) => name,
        None => return false,
    };

    let path = std::path::Path::new(cmd_name);
    if path.is_absolute() || path.exists() {
        return true;
    }

    if let Ok(path_env) = std::env::var("PATH") {
        for p in std::env::split_paths(&path_env) {
            let full_path = p.join(cmd_name);
            if full_path.exists() {
                return true;
            }
            if cfg!(target_os = "windows") {
                for ext in &["exe", "bat", "cmd", "com"] {
                    if full_path.with_extension(ext).exists() {
                        return true;
                    }
                }
            }
        }
    }
    false
}
