use crate::app::state::{AppState, TreeNode};
use crate::config::localization::t;
use std::path::Path;
use unicode_width::UnicodeWidthStr;

/// Builds info panel lines for the currently highlighted entry.
pub fn build_info_panel_lines(state: &AppState) -> Vec<String> {
    let panel = state.get_active_panel();
    let labels: Vec<String> = INFO_LABELS.iter().map(|key| t(key)).collect();
    let width = labels.iter().map(|l| l.width()).max().unwrap_or(0);
    let row = |label: usize, value: String| {
        let name = &labels[label];
        format!("{name}{} : {value}", " ".repeat(width - name.width()))
    };
    let continuation = |value: String| format!("{} : {value}", " ".repeat(width));
    let mut lines = Vec::new();

    if let Some(entry) = panel.entries.get(panel.cursor_index) {
        lines.push(row(NAME, entry.name.clone()));
        let kind = if entry.is_dir {
            t("info_panel_directory")
        } else {
            t("info_panel_file")
        };
        lines.push(row(KIND, kind));

        if !entry.is_dir {
            lines.push(row(
                SIZE,
                t("info_panel_bytes").replacen("{}", &entry.size.to_string(), 1),
            ));
            if entry.size >= 1024 {
                lines.push(continuation(format!(
                    "{:.2} KB",
                    entry.size as f64 / 1024.0
                )));
            }
            if entry.size >= 1024 * 1024 {
                lines.push(continuation(format!(
                    "{:.2} MB",
                    entry.size as f64 / (1024.0 * 1024.0)
                )));
            }
        }

        if let Some(modified) = entry.modified {
            let datetime: chrono::DateTime<chrono::Local> = modified.into();
            lines.push(row(
                MODIFIED,
                datetime.format("%Y-%m-%d %H:%M:%S").to_string(),
            ));
        }

        lines.push(String::new());
        lines.push(row(PATH, entry.path.to_string_lossy().into_owned()));
    }

    lines.push(String::new());
    lines.push(row(DIR, panel.current_path.to_string_lossy().into_owned()));

    let (total_files, total_dirs, total_size) = panel.entry_totals();
    lines.push(row(FILES, total_files.to_string()));
    lines.push(row(FOLDERS, total_dirs.to_string()));
    lines.push(row(
        TOTAL,
        format!("{:.2} MB", total_size as f64 / (1024.0 * 1024.0)),
    ));
    lines.push(String::new());
    lines.push(t("info_panel_close"));
    lines
}

/// Localization keys of the info panel labels, indexed by the constants below.
const INFO_LABELS: [&str; 9] = [
    "info_panel_name",
    "info_panel_type",
    "info_panel_size",
    "info_panel_modified",
    "info_panel_path",
    "info_panel_dir",
    "info_panel_files",
    "info_panel_folders",
    "info_panel_total",
];
const NAME: usize = 0;
const KIND: usize = 1;
const SIZE: usize = 2;
const MODIFIED: usize = 3;
const PATH: usize = 4;
const DIR: usize = 5;
const FILES: usize = 6;
const FOLDERS: usize = 7;
const TOTAL: usize = 8;

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
                // Only the top call adds its root node, so the children
                // are all entries (skipping one dropped each folder's first).
                nodes.extend(build_tree_nodes(&path, depth + 1, max_depth));
            }
        }
    }

    nodes
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn info_panel_labels_come_from_the_translations_and_align() {
        let state = AppState::new(PathBuf::from("."), PathBuf::from("."));
        let lines = build_info_panel_lines(&state);
        let dir = lines
            .iter()
            .find(|l| l.contains(&t("info_panel_dir")))
            .unwrap();
        let total = lines
            .iter()
            .find(|l| l.contains(&t("info_panel_total")))
            .unwrap();
        assert_eq!(dir.find(" : "), total.find(" : "), "colons line up");
        assert_eq!(lines.last().unwrap(), &t("info_panel_close"));
    }
}
