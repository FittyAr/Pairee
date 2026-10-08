use crate::app::context::AppContext;
use crate::app::form::FieldKey;
use crate::app::state::popup::GitPromptPopup;
use crate::app::state::{AppState, PopupType};
use crate::config::localization::t;
use crate::keybindings::actions::Action;
use crossterm::event::KeyEvent;

pub fn handle_clone(
    state: &mut AppState,
    key: KeyEvent,
    context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    let Some(PopupType::GitPrompt(GitPromptPopup::ClonePrompt(clone))) = state.dialogs.top_mut()
    else {
        return Err(());
    };
    match clone.fields.handle_key(&key) {
        FieldKey::Cancel => state.dialogs.clear(),
        FieldKey::Submit => {
            let url = clone.fields.first().trim().to_string();
            if url.is_empty() {
                return Ok(None);
            }
            let dir_name = match clone.fields.second().trim() {
                "" => repo_dir_name(&url),
                typed => typed.to_string(),
            };
            let target = clone.target_parent_path.join(dir_name);
            state.dialogs.clear();
            if target.exists() {
                state
                    .dialogs
                    .replace(PopupType::Error(t("git_clone_dir_exists")));
                return Ok(None);
            }
            state.start_git_op(
                crate::app::git_ops::GitNetOp::Clone { url, target },
                crate::app::git_ops::FollowUp::Cloned {
                    show_hidden: context.config.settings.show_hidden,
                },
            );
        }
        FieldKey::Handled | FieldKey::Other => {}
    }
    Ok(None)
}

/// Folder name derived from a clone URL (`.../name.git` → `name`).
fn repo_dir_name(url: &str) -> String {
    let clean = url.trim_end_matches('/').trim_end_matches(".git");
    match clean.split(['/', ':', '\\']).next_back() {
        Some(last) if !last.is_empty() => last.to_string(),
        _ => "repo".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::repo_dir_name;

    #[test]
    fn dir_name_from_url() {
        assert_eq!(repo_dir_name("https://h/o/pairee.git"), "pairee");
        assert_eq!(repo_dir_name("git@h:o/x/"), "x");
        assert_eq!(repo_dir_name("https://h/"), "h");
        assert_eq!(repo_dir_name(""), "repo");
    }
}
