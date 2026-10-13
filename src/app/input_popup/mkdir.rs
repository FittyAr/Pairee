use crate::app::context::AppContext;
use crate::app::form::FormKey;
use crate::app::state::popup::ConfirmRetryAsAdmin;
use crate::app::state::popup::MkDirPrompt;
use crate::app::state::popup::forms::MKDIR_FORM;
use crate::app::state::popup::{CreateKind, split_multiple};
use crate::app::state::popup::{Dialog, KeyHandler};
use crate::app::state::{AppState, PopupType};
use crate::app::text_input::TextField;
use crate::config::localization::t;
use crate::keybindings::Action;
use crossterm::event::KeyEvent;

const ROW_MULTIPLE: usize = 1;
const BUTTON_CANCEL: usize = 3;

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    let Some(PopupType::MkDirPrompt(MkDirPrompt {
        input,
        cursor_idx,
        process_multiple,
        kind,
    })) = state.dialogs.top_mut()
    else {
        return Err(());
    };
    let field = (*cursor_idx == 0).then_some(&mut *input);
    match MKDIR_FORM.handle(cursor_idx, field, &key) {
        FormKey::Toggle(ROW_MULTIPLE) => *process_multiple = !*process_multiple,
        FormKey::Activate(BUTTON_CANCEL) | FormKey::Cancel => state.dialogs.clear(),
        FormKey::Activate(_) => {
            let text = input.text().to_string();
            // "Process multiple names": `a;b;c` creates each of them.
            let names: Vec<&str> = if *process_multiple {
                split_multiple(&text).collect()
            } else {
                vec![text.as_str()]
            };
            let kind = *kind;
            for name in names {
                let (name, kind) = resolve(name, kind);
                let created = match kind {
                    CreateKind::File => create_file(state, context, &name),
                    _ => create(state, context, &name),
                };
                if !created {
                    break;
                }
            }
        }
        FormKey::Toggle(_) | FormKey::Handled | FormKey::Other => {}
    }
    Ok(None)
}

/// The name to create and whether it is a folder: `create` makes a folder
/// of a name ending with a path separator.
fn resolve(text: &str, kind: CreateKind) -> (String, CreateKind) {
    let name = text.trim();
    match kind {
        CreateKind::Auto if name.ends_with(['/', '\\']) => (
            name.trim_end_matches(['/', '\\']).to_string(),
            CreateKind::Folder,
        ),
        CreateKind::Auto => (name.to_string(), CreateKind::File),
        other => (name.to_string(), other),
    }
}

/// Creates an empty file `name` in the active panel; `false` when an error
/// is shown.
fn create_file(state: &mut AppState, context: &AppContext, name: &str) -> bool {
    state.dialogs.clear();
    if name.is_empty() {
        return true;
    }
    let panel = state.get_active_panel();
    let path = panel.current_path.join(name);
    if !panel.source.is_local() {
        let vfs = panel.source.vfs();
        state.start_vfs_op(move || vfs.write_file(&path, &mut std::io::empty()));
        return true;
    }
    match crate::fs::create_empty_file(&path) {
        Err(e) => {
            state
                .dialogs
                .replace(PopupType::error(format!("{} {}", t("error_dir_error"), e)));
            false
        }
        Ok(()) => {
            state
                .journal
                .record(crate::fs::journal::FsCommand::MakeFile { path });
            state.refresh_both_panels(context.config.settings.show_hidden);
            true
        }
    }
}

/// Creates the folder `name` in the active panel (offering an elevated
/// retry on failure); `false` when an error is shown.
fn create(state: &mut AppState, context: &AppContext, name: &str) -> bool {
    if name.is_empty() {
        state.dialogs.clear();
        return true;
    }
    let settings = &context.config.settings;
    let panel = state.get_active_panel();
    let path = panel.current_path.join(name);
    if !panel.source.is_local() {
        // SFTP server or zip archive: created in the background.
        let vfs = panel.source.vfs();
        state.dialogs.clear();
        state.start_vfs_op(move || vfs.mkdir(&path));
        return true;
    }
    match crate::fs::create_directory(&path, settings.req_admin_modification) {
        Err(_) if !settings.req_admin_modification => {
            state.dialogs.replace(ConfirmRetryAsAdmin {
                paths: vec![path],
                op_kind: crate::app::state::AdminOpKind::MkDir,
            });
            false
        }
        Err(e) => {
            state
                .dialogs
                .replace(PopupType::error(format!("{} {}", t("error_dir_error"), e)));
            false
        }
        Ok(()) => {
            state
                .journal
                .record(crate::fs::journal::FsCommand::MakeDir { path });
            if settings.req_admin_modification {
                state.redraw.needs_clear = true;
            }
            state.dialogs.clear();
            state.refresh_both_panels(settings.show_hidden);
            true
        }
    }
}

impl Dialog for MkDirPrompt {
    fn key_handler(&self) -> KeyHandler {
        handle
    }
    fn focused_field_mut(&mut self) -> Option<&mut TextField> {
        (self.cursor_idx == 0).then_some(&mut self.input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_makes_a_folder_of_a_trailing_separator() {
        assert_eq!(
            resolve("docs/", CreateKind::Auto),
            ("docs".into(), CreateKind::Folder)
        );
        assert_eq!(
            resolve(" a.txt ", CreateKind::Auto),
            ("a.txt".into(), CreateKind::File)
        );
        assert_eq!(
            resolve("x/", CreateKind::File),
            ("x/".into(), CreateKind::File)
        );
    }
}
