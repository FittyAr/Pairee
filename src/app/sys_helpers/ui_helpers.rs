use crate::app::state::{AppState, TreeNode};
use std::path::Path;

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
