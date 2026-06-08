use crate::app::context::AppContext;
use crate::app::state::{AppState, ProcessEntry, TreeNode};
use std::path::{Path, PathBuf};

/// Suspends raw mode in-place and kills the specified process by PID.
pub fn kill_process(pid: u32) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        let output = std::process::Command::new("kill")
            .arg("-9")
            .arg(pid.to_string())
            .output()?;
        if output.status.success() {
            Ok(())
        } else {
            let err_msg = String::from_utf8_lossy(&output.stderr);
            Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("kill failed: {}", err_msg),
            ))
        }
    }
    #[cfg(not(unix))]
    {
        let output = std::process::Command::new("taskkill")
            .arg("/F")
            .arg("/PID")
            .arg(pid.to_string())
            .output()?;
        if output.status.success() {
            Ok(())
        } else {
            let err_msg = String::from_utf8_lossy(&output.stderr);
            Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("taskkill failed: {}", err_msg),
            ))
        }
    }
}

/// Returns a list of running OS processes.
/// On Linux reads from /proc; on other platforms returns an empty list.
pub fn get_process_list() -> Vec<ProcessEntry> {
    #[allow(unused_mut)]
    let mut processes = Vec::new();

    #[cfg(target_os = "linux")]
    {
        if let Ok(read_dir) = std::fs::read_dir("/proc") {
            for entry in read_dir.flatten() {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                // /proc/<pid> directories have purely numeric names
                if let Ok(pid) = name_str.parse::<u32>() {
                    let comm_path = entry.path().join("comm");
                    let proc_name = std::fs::read_to_string(&comm_path)
                        .unwrap_or_default()
                        .trim()
                        .to_string();
                    // Read VmRSS from status for memory approximation
                    let memory_kb = read_proc_memory(pid);
                    processes.push(ProcessEntry {
                        pid,
                        name: if proc_name.is_empty() {
                            format!("[{}]", pid)
                        } else {
                            proc_name
                        },
                        memory_kb,
                    });
                }
            }
        }
        processes.sort_by_key(|p| p.pid);
    }

    processes
}

#[cfg(target_os = "linux")]
fn read_proc_memory(pid: u32) -> u64 {
    let status_path = format!("/proc/{}/status", pid);
    if let Ok(content) = std::fs::read_to_string(&status_path) {
        for line in content.lines() {
            if line.starts_with("VmRSS:") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if let Some(kb_str) = parts.get(1) {
                    return kb_str.parse::<u64>().unwrap_or(0);
                }
            }
        }
    }
    0
}

/// Returns the system drives/mounts for the current OS.
pub fn get_system_drives() -> Vec<String> {
    let mut drives = Vec::new();
    if cfg!(target_os = "windows") {
        for drive_letter in b'A'..=b'Z' {
            let path = format!("{}:\\", drive_letter as char);
            if std::path::Path::new(&path).exists() {
                drives.push(path);
            }
        }
    } else {
        let paths = vec!["/", "/home", "/media", "/mnt", "/tmp"];
        for p in paths {
            if std::path::Path::new(p).exists() {
                drives.push(p.to_string());
            }
        }
    }
    if drives.is_empty() {
        drives.push("/".to_string());
    }
    drives
}

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

/// Changes the current configuration theme.
pub fn change_theme(context: &mut AppContext, state: &mut AppState, theme_name: &str) {
    context.config.settings.theme = theme_name.to_string();
    let theme = if theme_name == "classic_blue" {
        crate::config::theme::Theme::classic_blue()
    } else {
        crate::config::theme::Theme::default()
    };
    context.config.theme = theme;
    let _ = context.config.save();
    state.refresh_both_panels(context.config.settings.show_hidden);
}

/// Changes the current keybinding preset.
pub fn change_preset(context: &mut AppContext, preset_name: &str) {
    context.config.keybindings.preset = preset_name.to_string();
    context.config.settings.keybinding_preset = preset_name.to_string();
    context.resolver = crate::keybindings::KeybindingResolver::new(&context.config);
    let _ = context.config.save();
}

/// Builds info panel lines for the currently highlighted entry.
pub fn build_info_panel_lines(state: &AppState) -> Vec<String> {
    let panel = state.get_active_panel();
    let mut lines = Vec::new();

    if let Some(entry) = panel.entries.get(panel.cursor_index) {
        lines.push(format!("Name    : {}", entry.name));
        lines.push(format!(
            "Type    : {}",
            if entry.is_dir { "Directory" } else { "File" }
        ));

        if !entry.is_dir {
            lines.push(format!("Size    : {} bytes", entry.size));
            if entry.size >= 1024 {
                lines.push(format!("        : {:.2} KB", entry.size as f64 / 1024.0));
            }
            if entry.size >= 1024 * 1024 {
                lines.push(format!(
                    "        : {:.2} MB",
                    entry.size as f64 / (1024.0 * 1024.0)
                ));
            }
        }

        if let Some(modified) = entry.modified {
            let datetime: chrono::DateTime<chrono::Local> = modified.into();
            lines.push(format!(
                "Modified: {}",
                datetime.format("%Y-%m-%d %H:%M:%S")
            ));
        }

        lines.push(String::new());
        lines.push(format!("Path    : {}", entry.path.to_string_lossy()));
    }

    lines.push(String::new());
    lines.push(format!(
        "Dir     : {}",
        panel.current_path.to_string_lossy()
    ));

    let total_files = panel.entries.iter().filter(|e| !e.is_dir).count();
    let total_dirs = panel
        .entries
        .iter()
        .filter(|e| e.is_dir && e.name != "..")
        .count();
    let total_size: u64 = panel
        .entries
        .iter()
        .filter(|e| !e.is_dir)
        .map(|e| e.size)
        .sum();

    lines.push(format!("Files   : {}", total_files));
    lines.push(format!("Folders : {}", total_dirs));
    lines.push(format!(
        "Total   : {:.2} MB",
        total_size as f64 / (1024.0 * 1024.0)
    ));
    lines.push(String::new());
    lines.push("[Enter/Esc] Close".to_string());
    lines
}

/// Recursively builds tree nodes for the graphical tree navigator feature.
pub fn build_tree_nodes(root: &Path, depth: usize, max_depth: usize) -> Vec<TreeNode> {
    let mut nodes = Vec::new();

    if depth == 0 {
        nodes.push(TreeNode {
            depth: 0,
            name: root
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| root.to_string_lossy().to_string()),
            path: root.to_path_buf(),
            is_dir: true,
        });
    }

    if depth >= max_depth {
        return nodes;
    }

    if let Ok(read_dir) = std::fs::read_dir(root) {
        let mut entries: Vec<_> = read_dir.flatten().collect();
        entries.sort_by_key(|e| {
            let is_file = e.file_type().map(|ft| !ft.is_dir()).unwrap_or(false);
            (is_file, e.file_name())
        });

        for entry in entries {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            // Skip hidden files/system dirs
            if name.starts_with('.') {
                continue;
            }
            let is_dir = path.is_dir();

            nodes.push(TreeNode {
                depth: depth + 1,
                name: name.clone(),
                path: path.clone(),
                is_dir,
            });

            if is_dir && depth + 1 < max_depth {
                let children = build_tree_nodes(&path, depth + 1, max_depth);
                // Skip the root node of each recursive call (first element is the dir itself)
                nodes.extend(children.into_iter().skip(1));
            }
        }
    }

    nodes
}

/// Recursive file search — returns paths whose filenames contain `query` (case-insensitive)
/// and optionally contain the requested search content.
pub fn search_files_recursive(
    root: &Path,
    query: &str,
    content_query: Option<&str>,
) -> Vec<PathBuf> {
    let name_glob = if query.is_empty() {
        "".to_string()
    } else if query.contains('*') || query.contains('?') {
        query.to_string()
    } else {
        format!("*{}*", query)
    };

    let q = crate::fs::search::SearchQuery {
        name_glob,
        content: content_query.map(|s| s.to_string()),
        root: root.to_path_buf(),
    };

    let mut rx = crate::fs::search::find_files(q);
    let mut results = Vec::new();
    while let Some(path) = rx.blocking_recv() {
        results.push(path);
        if results.len() >= 500 {
            break;
        }
    }
    results
}

/// Searches for the next occurrence of `query` in the editor.
pub fn find_next_in_editor(
    lines: &[String],
    current_x: usize,
    current_y: usize,
    query: &str,
) -> Option<(usize, usize)> {
    if query.is_empty() || lines.is_empty() {
        return None;
    }
    let q_lower = query.to_lowercase();

    // 1. Search current line forward (starting at current_x + 1)
    if current_y < lines.len() {
        let line = &lines[current_y];
        let start_idx = current_x + 1;
        if start_idx < line.len() {
            if let Some(pos) = line[start_idx..].to_lowercase().find(&q_lower) {
                return Some((start_idx + pos, current_y));
            }
        }
    }

    // 2. Search subsequent lines forward
    for y in (current_y + 1)..lines.len() {
        if let Some(pos) = lines[y].to_lowercase().find(&q_lower) {
            return Some((pos, y));
        }
    }

    // 3. Wrap around: Search from start of file up to current_y
    for y in 0..=current_y {
        let line = &lines[y];
        let limit = if y == current_y {
            current_x
        } else {
            line.len()
        };
        if let Some(pos) = line[..limit].to_lowercase().find(&q_lower) {
            return Some((pos, y));
        }
    }

    None
}

/// Refreshes the current process environment variables from the registry on Windows.
pub fn refresh_env_vars() {
    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        let cmd = "[Environment]::GetEnvironmentVariables('Machine').GetEnumerator() | % { \"$($_.Key)=$($_.Value)\" }; [Environment]::GetEnvironmentVariables('User').GetEnumerator() | % { \"$($_.Key)=$($_.Value)\" }";
        if let Ok(output) = Command::new("powershell")
            .args(&["-NoProfile", "-Command", cmd])
            .output()
        {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                for line in stdout.lines() {
                    if let Some(pos) = line.find('=') {
                        let key = &line[..pos];
                        let val = &line[pos + 1..];
                        unsafe {
                            std::env::set_var(key, val);
                        }
                    }
                }
            }
        }
    }
}

/// Returns available free space in bytes for the volume containing `path`.
/// Uses native Win32 `GetDiskFreeSpaceExW` on Windows; reads /proc/mounts on other platforms.
/// Returns `None` if the query fails.
pub fn get_free_space(path: &std::path::Path) -> Option<u64> {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::ffi::OsStrExt;
        let wide: Vec<u16> = path
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let mut free_bytes: u64 = 0;
        let mut _total_bytes: u64 = 0;
        let mut _total_free: u64 = 0;
        // SAFETY: We pass valid non-null pointers for output parameters.
        let ret = unsafe {
            GetDiskFreeSpaceExW(
                wide.as_ptr(),
                &mut free_bytes,
                &mut _total_bytes,
                &mut _total_free,
            )
        };
        if ret != 0 {
            return Some(free_bytes);
        }
        return None;
    }
    #[cfg(not(target_os = "windows"))]
    {
        // Use `df` command as a portable cross-platform fallback
        let output = std::process::Command::new("df")
            .arg("--output=avail")
            .arg("-k")
            .arg(path)
            .output()
            .ok()?;
        let text = String::from_utf8_lossy(&output.stdout);
        let kb: u64 = text.lines().nth(1)?.trim().parse().ok()?;
        Some(kb * 1024)
    }
}

#[cfg(target_os = "windows")]
unsafe extern "system" {
    fn GetDiskFreeSpaceExW(
        lp_directory_name: *const u16,
        lp_free_bytes_available_to_caller: *mut u64,
        lp_total_number_of_bytes: *mut u64,
        lp_total_number_of_free_bytes: *mut u64,
    ) -> i32;
}
