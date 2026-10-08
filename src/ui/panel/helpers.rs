use crate::app::state::{PanelState, PanelViewMode, SortField};
use crate::config::localization::t;
use crate::config::settings::Settings;
use crate::fs::FileEntry;
use crate::ui::text_width::truncate_to_width;
use std::time::SystemTime;

pub(crate) fn build_panel_title(panel: &PanelState, settings: &Settings) -> String {
    let mode_label = match panel.view_mode {
        PanelViewMode::Brief => t("panel_mode_brief"),
        PanelViewMode::Medium => t("panel_mode_medium"),
        PanelViewMode::Full => t("panel_mode_full"),
        PanelViewMode::Wide => t("panel_mode_wide"),
        PanelViewMode::Detailed => t("panel_mode_detailed"),
        PanelViewMode::Descriptions => t("panel_mode_desc"),
        PanelViewMode::FileOwners => t("panel_mode_owners"),
        PanelViewMode::FileLinks => t("panel_mode_links"),
        PanelViewMode::AltFull => t("panel_mode_alt"),
    };

    let sort_letter = if settings.show_sort_mode_letter {
        let letter = match panel.sort_field {
            SortField::Name => "N",
            SortField::Extension => "X",
            SortField::Size => "S",
            SortField::Date => "D",
            SortField::Unsorted => "U",
        };
        let rev = if panel.sort_reverse { "▼" } else { "▲" };
        format!("|{}{}", letter, rev)
    } else {
        String::new()
    };

    let ssh_suffix = if let Some(client) = &panel.ssh_conn {
        let info = client.info();
        format!(" [SSH: {}@{}]", info.username, info.host)
    } else {
        String::new()
    };

    let git_suffix = if settings.git_enabled {
        if let Some(ref branch) = panel.git_branch {
            format!(" [git: {}]", branch)
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    let loading = if panel.is_loading() {
        format!(" {}", t("panel_loading"))
    } else if panel.dir_sizes.is_running() {
        format!(" {}", t("dir_size_title_running"))
    } else {
        String::new()
    };

    format!(
        " {}{}{} [{}{}]{} ",
        panel.current_path.to_string_lossy(),
        ssh_suffix,
        git_suffix,
        mode_label,
        sort_letter,
        loading,
    )
}

pub(crate) fn visible_range(panel: &PanelState, height: usize) -> (usize, usize) {
    let start = if panel.cursor_index > height / 2 {
        panel.cursor_index.saturating_sub(height / 2)
    } else {
        0
    };
    let start = if start + height > panel.entries.len() {
        panel.entries.len().saturating_sub(height)
    } else {
        start
    };
    (
        start,
        start + height.min(panel.entries.len().saturating_sub(start)),
    )
}

/// Entries in `start..end`, or nothing when the range is out of bounds
/// (never panics while drawing, even if entries changed underneath).
pub(crate) fn visible_slice(panel: &PanelState, start: usize, end: usize) -> &[FileEntry] {
    panel.entries.get(start..end).unwrap_or_default()
}

pub(crate) fn entry_display_name(name: &str, is_dir: bool, git_status: Option<&str>) -> String {
    let prefix = match git_status {
        Some(st) => format!("[{}] ", st),
        None => String::new(),
    };
    if is_dir && name != ".." {
        format!("{}/{}", prefix, name)
    } else {
        format!("{}{}", prefix, name)
    }
}

/// Display name truncated to `max_width` terminal columns (Unicode-aware).
pub(crate) fn entry_display_name_truncated(
    name: &str,
    is_dir: bool,
    max_width: usize,
    git_status: Option<&str>,
) -> String {
    truncate_to_width(&entry_display_name(name, is_dir, git_status), max_width)
}

pub(crate) fn format_file_size(size: u64) -> String {
    if size < 1024 {
        format!("{} B", size)
    } else if size < 1024 * 1024 {
        format!("{:.1} KB", size as f64 / 1024.0)
    } else if size < 1024 * 1024 * 1024 {
        format!("{:.1} MB", size as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.1} GB", size as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

/// A computed folder size, with the "incomplete" marker when part of the
/// tree could not be read.
pub(crate) fn dir_size_text(size: &crate::fs::du::DirSize) -> String {
    let text = format_file_size(size.bytes);
    if size.partial {
        t("dir_size_partial").replace("{}", &text)
    } else {
        text
    }
}

/// Size text for `entry`: the file size, the computed folder size (marked
/// when partial or still being measured), or `None` for an unmeasured folder.
pub(crate) fn entry_size_text(panel: &PanelState, entry: &FileEntry) -> Option<String> {
    if !entry.is_dir {
        return Some(format_file_size(entry.size));
    }
    if let Some(size) = panel.dir_sizes.get(&entry.path) {
        return Some(dir_size_text(size));
    }
    panel
        .dir_sizes
        .progress()
        .filter(|p| p.target == entry.path)
        .map(|p| t("dir_size_running").replace("{}", &format_file_size(p.bytes)))
}

pub(crate) fn format_date(time: Option<SystemTime>) -> String {
    match time {
        Some(t) => {
            let dt: chrono::DateTime<chrono::Local> = t.into();
            dt.format("%d/%m/%Y %H:%M").to_string()
        }
        None => String::new(),
    }
}

/// Free space computed by the last listing (never queried while drawing).
pub(crate) fn free_space_text(free: Option<u64>) -> String {
    match free {
        Some(bytes) => format_file_size(bytes),
        None => "?".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_build_panel_title_git() {
        let mut panel = PanelState::new(PathBuf::from("/test/repo"));
        let mut settings = Settings {
            git_enabled: true,
            ..Default::default()
        };

        let title_no_git = build_panel_title(&panel, &settings);
        assert!(!title_no_git.contains("[git:"));

        panel.git_branch = Some("feature-xyz".to_string());
        let title_with_git = build_panel_title(&panel, &settings);
        assert!(title_with_git.contains("[git: feature-xyz]"));

        settings.git_enabled = false;
        let title_disabled = build_panel_title(&panel, &settings);
        assert!(!title_disabled.contains("[git:"));
    }

    #[test]
    fn test_entry_display_name_git_status() {
        assert_eq!(entry_display_name("file.txt", false, None), "file.txt");
        assert_eq!(
            entry_display_name("file.txt", false, Some("M")),
            "[M] file.txt"
        );
        assert_eq!(entry_display_name("sub", true, Some("?")), "[?] /sub");
        assert_eq!(entry_display_name("..", true, None), "..");
    }

    #[test]
    fn entry_size_text_uses_computed_folder_sizes() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("sub")).unwrap();
        std::fs::write(dir.path().join("sub/f"), [0u8; 10]).unwrap();
        let mut panel = PanelState::new(dir.path().to_path_buf());
        let folder = FileEntry {
            name: "sub".into(),
            path: dir.path().join("sub"),
            size: 0,
            is_dir: true,
            is_symlink: false,
            modified: None,
        };
        panel.entries = vec![folder.clone()];
        assert_eq!(entry_size_text(&panel, &folder), None);
        panel.calculate_dir_sizes(std::slice::from_ref(&folder.path));
        panel.dir_sizes.poll();
        assert_eq!(entry_size_text(&panel, &folder).as_deref(), Some("10 B"));
    }
}
