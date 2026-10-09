use super::report::KeymapLoadReport;
use crate::config::paths;
use crate::keybindings::embedded::{self, normalize_preset_name};
use std::path::{Path, PathBuf};

/// The TOML of preset `name`: the user's `keymaps/` folder first, then the
/// `keymaps/` shipped next to the binary, then the copy embedded in it.
/// `None` when no preset has that name.
pub fn find_preset_toml(name: &str, report: &mut KeymapLoadReport) -> Option<String> {
    let name = normalize_preset_name(name);
    if let Some(user) = read_user_preset(&paths::get_keymaps_dir(), &name, report) {
        return Some(user);
    }
    shipped_keymap_candidates(&name)
        .into_iter()
        .find_map(|candidate| std::fs::read_to_string(candidate).ok())
        .or_else(|| embedded::preset_toml(&name).map(str::to_string))
}

/// `keymaps_dir/<name>.toml` when present, warning when the seeder left a
/// newer shipped version next to it.
fn read_user_preset(
    keymaps_dir: &Path,
    name: &str,
    report: &mut KeymapLoadReport,
) -> Option<String> {
    let path = keymaps_dir.join(format!("{name}.toml"));
    if !path.exists() {
        return None;
    }
    let pending = embedded::pending_update_path(keymaps_dir, name);
    if pending.exists() {
        report.warnings.push(format!(
            "'{}' differs from the shipped preset and was kept; the new version is in '{}'",
            path.display(),
            pending.display()
        ));
    }
    match std::fs::read_to_string(&path) {
        Ok(s) => Some(s),
        Err(e) => {
            report
                .warnings
                .push(format!("Could not read '{}': {e}", path.display()));
            None
        }
    }
}

fn shipped_keymap_candidates(name: &str) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(cwd) = std::env::current_dir() {
        out.push(cwd.join("keymaps").join(format!("{name}.toml")));
    }
    if let Ok(exe) = std::env::current_exe()
        && let Some(dir) = exe.parent()
    {
        out.push(dir.join("keymaps").join(format!("{name}.toml")));
        out.push(
            dir.join("../share/pairee/keymaps")
                .join(format!("{name}.toml")),
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_preset_warns_about_a_pending_shipped_update() {
        let dir = tempfile::tempdir().unwrap();
        let mut report = KeymapLoadReport::default();
        assert_eq!(read_user_preset(dir.path(), "norton", &mut report), None);
        std::fs::write(
            dir.path().join("norton.toml"),
            "[bindings]
",
        )
        .unwrap();
        assert!(read_user_preset(dir.path(), "norton", &mut report).is_some());
        assert!(report.warnings.is_empty());
        std::fs::write(embedded::pending_update_path(dir.path(), "norton"), "").unwrap();
        read_user_preset(dir.path(), "norton", &mut report);
        assert!(report.warnings[0].contains("norton.toml.new"));
    }
}
