use crate::app::context::AppContext;
use crate::app::state::popup::ConfirmRetryAsAdmin;
use crate::app::state::popup::RenamePrompt;
use crate::app::state::{AppState, PopupType};
use crate::app::text_input::TextField;
use crate::config::localization::t;
use crate::fs::journal::FsCommand;
use crate::fs::multi_rename::{Step, TargetFs};
use unicode_segmentation::UnicodeSegmentation;

/// Opens the rename prompt; with `basename` the cursor starts before the
/// extension (Vim's `cW`, yazi's `r`).
pub fn handle(state: &mut AppState, _context: &mut AppContext, basename: bool) -> bool {
    let active = state.get_active_panel();
    if let Some(entry) = active.entries.get(active.cursor_index) {
        // A branch view names entries by relative path: rename the file
        // where it is.
        let (original, parent_dir) = match entry.path.parent() {
            Some(parent) if active.branch_view => (
                crate::fs::file_name_lossy(&entry.path),
                parent.to_path_buf(),
            ),
            _ => (entry.name.clone(), active.current_path.clone()),
        };
        let mut input = TextField::new(original.as_str());
        if basename {
            for _ in 0..extension_len(&original) {
                input.move_left();
            }
        }
        state.dialogs.replace(RenamePrompt {
            input,
            original,
            src_path: entry.path.clone(),
            parent_dir,
            cursor_idx: 0,
        });
        true
    } else {
        state
            .dialogs
            .replace(PopupType::error(t("error_no_entry_rename")));
        true
    }
}

/// Graphemes (cursor steps) of `.ext` in `name` (none for dot files like `.bashrc`).
fn extension_len(name: &str) -> usize {
    match name.rfind('.') {
        Some(0) | None => 0,
        Some(dot) => name[dot..].graphemes(true).count(),
    }
}

/// Perform the actual rename on Enter. Extracted so the input handler
/// can call it after the user confirms the new filename.
pub fn commit(
    state: &mut AppState,
    context: &mut AppContext,
    input: String,
    original: String,
    src_path: std::path::PathBuf,
    parent_dir: std::path::PathBuf,
) {
    let trimmed = input.trim().to_string();
    if trimmed.is_empty() || trimmed == original {
        state.dialogs.clear();
        return;
    }
    let remote = state.get_active_panel().source.remote().cloned();
    // A name with a separator (or `.`, `..`) would move the file elsewhere.
    if !TargetFs::for_panel(remote.is_some()).is_valid_name(&trimmed) {
        state.dialogs.replace(PopupType::error(format!(
            "{} {}",
            t("error_rename_error"),
            t("error_rename_invalid_name")
        )));
        return;
    }
    let target = parent_dir.join(&trimmed);
    if target == src_path {
        state.dialogs.clear();
        return;
    }
    if let Some(remote) = remote {
        // SFTP: renamed in the background by the rename executor, which
        // refuses an existing target and journals the step for undo.
        state.dialogs.clear();
        let step = Step {
            from: src_path,
            to: target,
        };
        super::multi_rename::run_steps(state, vec![step], Some(remote), None);
        return;
    }
    // `rename` silently replaces an existing file: refuse instead (a
    // change of letter case on Windows names the same file and is fine).
    if is_taken(&src_path, &target) {
        state.dialogs.replace(PopupType::error(format!(
            "{} {}",
            t("error_rename_error"),
            t("prompt_rename_collision")
        )));
        return;
    }
    match std::fs::rename(&src_path, &target) {
        Ok(_) => {
            state.journal.record(FsCommand::Rename {
                steps: vec![Step {
                    from: src_path,
                    to: target,
                }],
                remote: None,
            });
            if context.config.settings.req_admin_modification {
                state.redraw.needs_clear = true;
            }
            state.dialogs.clear();
            state.refresh_both_panels(context.config.settings.show_hidden);
        }
        Err(e) => {
            let denied = e.kind() == std::io::ErrorKind::PermissionDenied;
            if denied && !context.config.settings.req_admin_modification {
                state.dialogs.replace(ConfirmRetryAsAdmin {
                    paths: vec![src_path.clone()],
                    op_kind: crate::app::state::AdminOpKind::Rename {
                        src: src_path,
                        target,
                    },
                });
            } else {
                state.dialogs.replace(PopupType::error(format!(
                    "{} {}",
                    t("error_rename_error"),
                    e
                )));
            }
        }
    }
}

/// Whether `target` exists as another object than `src`.
fn is_taken(src: &std::path::Path, target: &std::path::Path) -> bool {
    use crate::fs::file_id::file_identity;
    if std::fs::symlink_metadata(target).is_err() {
        return false;
    }
    match (file_identity(src, false), file_identity(target, false)) {
        (Ok(a), Ok(b)) => a != b,
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::{extension_len, is_taken};

    #[test]
    fn an_existing_target_is_taken_unless_it_is_the_same_file() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.txt");
        let b = dir.path().join("b.txt");
        std::fs::write(&a, "a").unwrap();
        assert!(!is_taken(&a, &b));
        std::fs::write(&b, "b").unwrap();
        assert!(is_taken(&a, &b));
        #[cfg(windows)]
        assert!(!is_taken(&a, &dir.path().join("A.TXT")), "letter case only");
    }

    #[test]
    fn extension_is_what_follows_the_last_dot() {
        assert_eq!(extension_len("report.tar.gz"), 3);
        assert_eq!(extension_len("Makefile"), 0);
        assert_eq!(extension_len(".bashrc"), 0, "a dot file has no extension");
        assert_eq!(extension_len("año.txt"), 4);
        assert_eq!(
            extension_len("e\u{301}.tx\u{301}t"),
            3,
            "combining marks are one step"
        );
    }
}
