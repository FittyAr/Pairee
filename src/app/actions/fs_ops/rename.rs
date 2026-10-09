use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType};
use crate::app::text_input::TextField;
use crate::config::localization::t;
use crate::fs::journal::FsCommand;
use crate::fs::multi_rename::Step;

/// Opens the rename prompt; with `basename` the cursor starts before the
/// extension (Vim's `cW`, yazi's `r`).
pub fn handle(state: &mut AppState, _context: &mut AppContext, basename: bool) -> bool {
    let active = state.get_active_panel();
    if let Some(entry) = active.entries.get(active.cursor_index) {
        let original = entry.name.clone();
        let mut input = TextField::new(original.as_str());
        if basename {
            for _ in 0..extension_len(&original) {
                input.move_left();
            }
        }
        state.dialogs.replace(PopupType::RenamePrompt {
            input,
            original,
            src_path: entry.path.clone(),
            parent_dir: active.current_path.clone(),
            cursor_idx: 0,
        });
        true
    } else {
        state
            .dialogs
            .replace(PopupType::Error(t("error_no_entry_rename")));
        true
    }
}

/// Characters of `.ext` in `name` (none for dot files like `.bashrc`).
fn extension_len(name: &str) -> usize {
    match name.rfind('.') {
        Some(0) | None => 0,
        Some(dot) => name[dot..].chars().count(),
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
    let target = parent_dir.join(&trimmed);
    if target == src_path {
        state.dialogs.clear();
        return;
    }
    if let Some(ssh) = state.get_active_panel().source.ssh().cloned() {
        // SFTP: renamed in the background by the rename executor, which
        // refuses an existing target and journals the step for undo.
        state.dialogs.clear();
        let step = Step {
            from: src_path,
            to: target,
        };
        super::multi_rename::run_steps(state, vec![step], Some(ssh));
        return;
    }
    match std::fs::rename(&src_path, &target) {
        Ok(_) => {
            state.journal.record(FsCommand::Rename {
                steps: vec![Step {
                    from: src_path,
                    to: target,
                }],
                ssh: None,
            });
            if context.config.settings.req_admin_modification {
                state.terminal_needs_clear = true;
            }
            state.dialogs.clear();
            state.refresh_both_panels(context.config.settings.show_hidden);
        }
        Err(e) => {
            if !context.config.settings.req_admin_modification {
                state.dialogs.replace(PopupType::ConfirmRetryAsAdmin {
                    paths: vec![src_path.clone()],
                    op_kind: crate::app::state::AdminOpKind::Rename {
                        src: src_path,
                        target,
                    },
                });
            } else {
                state.dialogs.replace(PopupType::Error(format!(
                    "{} {}",
                    t("error_rename_error"),
                    e
                )));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::extension_len;

    #[test]
    fn extension_is_what_follows_the_last_dot() {
        assert_eq!(extension_len("report.tar.gz"), 3);
        assert_eq!(extension_len("Makefile"), 0);
        assert_eq!(extension_len(".bashrc"), 0, "a dot file has no extension");
        assert_eq!(extension_len("año.txt"), 4);
    }
}
