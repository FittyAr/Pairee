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

    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        if let Ok(output) = Command::new("tasklist")
            .args(&["/FO", "CSV", "/NH"])
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                let parts: Vec<&str> = if line.starts_with('"') && line.ends_with('"') {
                    line[1..line.len() - 1].split("\",\"").collect()
                } else {
                    line.split(',').collect()
                };
                if parts.len() >= 5 {
                    let name = parts[0].to_string();
                    let pid = parts[1].parse::<u32>().unwrap_or(0);
                    let mem_str = parts[4];
                    let digits_only: String =
                        mem_str.chars().filter(|c| c.is_ascii_digit()).collect();
                    let memory_kb = digits_only.parse::<u64>().unwrap_or(0);
                    processes.push(ProcessEntry {
                        pid,
                        name,
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

/// Searches for the next occurrence of `query` in the editor.
pub fn find_next_in_editor(
    lines: &[String],
    current_x: usize,
    current_y: usize,
    query: &str,
    case_sensitive: bool,
) -> Option<(usize, usize)> {
    if query.is_empty() || lines.is_empty() {
        return None;
    }

    let match_fn = |text: &str, pat: &str| -> Option<usize> {
        if case_sensitive {
            text.find(pat)
        } else {
            text.to_lowercase().find(&pat.to_lowercase())
        }
    };

    // 1. Search current line forward (starting at current_x + 1)
    if current_y < lines.len() {
        let line = &lines[current_y];
        let start_idx = current_x + 1;
        if start_idx < line.len() {
            if let Some(pos) = match_fn(&line[start_idx..], query) {
                return Some((start_idx + pos, current_y));
            }
        }
    }

    // 2. Search subsequent lines forward
    for y in (current_y + 1)..lines.len() {
        if let Some(pos) = match_fn(&lines[y], query) {
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
        if let Some(pos) = match_fn(&line[..limit], query) {
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

use std::collections::BTreeMap;

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

#[cfg(target_os = "linux")]
fn get_process_restart_info(pid: u32) -> Option<(String, Vec<String>, Option<PathBuf>)> {
    let cmdline_bytes = std::fs::read(format!("/proc/{}/cmdline", pid)).ok()?;
    if cmdline_bytes.is_empty() {
        return None;
    }
    let mut args = Vec::new();
    let mut current = Vec::new();
    for &b in &cmdline_bytes {
        if b == 0 {
            if !current.is_empty() {
                if let Ok(s) = String::from_utf8(current.clone()) {
                    args.push(s);
                }
                current.clear();
            }
        } else {
            current.push(b);
        }
    }
    if !current.is_empty() {
        if let Ok(s) = String::from_utf8(current) {
            args.push(s);
        }
    }
    if args.is_empty() {
        return None;
    }
    let executable = args[0].clone();
    let remaining_args = args[1..].to_vec();
    let cwd = std::fs::read_link(format!("/proc/{}/cwd", pid)).ok();
    Some((executable, remaining_args, cwd))
}

#[cfg(target_os = "windows")]
fn get_process_restart_info(pid: u32) -> Option<(String, Vec<String>, Option<PathBuf>)> {
    use std::process::Command;
    let ps_cmd = format!(
        "Get-CimInstance Win32_Process -Filter 'ProcessId = {}' | ForEach-Object {{ $_.Path + '|' + $_.CommandLine + '|' + $_.WorkingDirectory }}",
        pid
    );
    let output = Command::new("powershell")
        .args(&["-Command", &ps_cmd])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let line = stdout.trim();
    if line.is_empty() {
        return None;
    }
    let parts: Vec<&str> = line.split('|').collect();
    if parts.len() >= 2 {
        let _executable = parts[0].to_string();
        let cmd_line = parts[1].to_string();
        let cwd_str = parts.get(2).map(|s| s.to_string());

        let executable = "cmd".to_string();
        let args = vec!["/C".to_string(), cmd_line];
        let cwd = cwd_str.filter(|s| !s.trim().is_empty()).map(PathBuf::from);

        Some((executable, args, cwd))
    } else {
        None
    }
}

#[cfg(all(unix, not(target_os = "linux")))]
fn get_process_restart_info(pid: u32) -> Option<(String, Vec<String>, Option<PathBuf>)> {
    use std::process::Command;
    let output = Command::new("ps")
        .args(&["-p", &pid.to_string(), "-o", "comm="])
        .output()
        .ok()?;
    if output.status.success() {
        let name = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !name.is_empty() {
            return Some((name, Vec::new(), None));
        }
    }
    None
}

pub fn restart_process(pid: u32) -> std::io::Result<()> {
    let info = get_process_restart_info(pid).ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Process info not found or already terminated",
        )
    })?;

    kill_process(pid)?;

    let mut cmd = std::process::Command::new(&info.0);
    cmd.args(&info.1);
    if let Some(cwd) = info.2 {
        cmd.current_dir(cwd);
    }
    cmd.spawn()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_next_in_editor() {
        let lines = vec![
            "The quick brown fox".to_string(),
            "jumps over the lazy dog".to_string(),
            "The end".to_string(),
        ];

        // Case insensitive search
        assert_eq!(
            find_next_in_editor(&lines, 0, 0, "the", false),
            Some((11, 1))
        );
        assert_eq!(
            find_next_in_editor(&lines, 11, 1, "the", false),
            Some((0, 2))
        );
        assert_eq!(
            find_next_in_editor(&lines, 0, 2, "the", false),
            Some((0, 0))
        ); // Wrap around

        // Case sensitive search
        assert_eq!(find_next_in_editor(&lines, 0, 0, "The", true), Some((0, 2)));
        assert_eq!(
            find_next_in_editor(&lines, 0, 0, "the", true),
            Some((11, 1))
        );
        assert_eq!(
            find_next_in_editor(&lines, 11, 1, "The", true),
            Some((0, 2))
        );
        assert_eq!(find_next_in_editor(&lines, 0, 2, "The", true), Some((0, 0))); // Wrap around
    }
}
