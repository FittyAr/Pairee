//! Copy (F5) and Move (F6): open the transfer dialog, or submit straight away
//! with the transfer settings when the confirmation is turned off.

use crate::app::context::AppContext;
use crate::app::state::popup::{CopyMovePromptState, TransferPromptOp};
use crate::app::state::{AppState, PopupType};
use crate::fs::transfer::{submit_simple, transfer_options_from_settings};

pub fn handle(state: &mut AppState, context: &mut AppContext, op: TransferPromptOp) -> bool {
    let targets = state.get_active_panel().get_targeted_paths();
    if targets.is_empty() {
        return true;
    }
    let dest_dir = state.get_passive_panel().current_path.clone();
    let confirmations = &context.config.settings.confirmations;
    let confirm = match op {
        TransferPromptOp::Copy => confirmations.confirm_copy,
        TransferPromptOp::Move => confirmations.confirm_move,
    };
    if confirm {
        state
            .dialogs
            .replace(PopupType::TransferPrompt(CopyMovePromptState::new(
                op, targets, dest_dir,
            )));
    } else {
        let options = transfer_options_from_settings(&context.config.settings);
        submit_simple(
            state,
            op.operation(),
            targets,
            dest_dir,
            options,
            state.get_active_panel().source.ssh().cloned(),
            state.get_passive_panel().source.ssh().cloned(),
        );
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;
    use crate::fs::transfer::job::TransferOperation;

    fn context() -> AppContext {
        AppContext::new(AppConfig::default())
    }

    #[tokio::test]
    async fn move_without_confirmation_uses_transfer_settings() {
        let src = tempfile::tempdir().unwrap();
        let dst = tempfile::tempdir().unwrap();
        std::fs::write(src.path().join("a.txt"), b"x").unwrap();
        let mut state = AppState::new(src.path().to_path_buf(), dst.path().to_path_buf());
        state
            .get_active_panel_mut()
            .selected_paths
            .insert(src.path().join("a.txt"));
        let mut context = context();
        context.config.settings.confirmations.confirm_move = false;
        context.config.settings.transfer_preserve_attributes = true;
        context.config.settings.transfer_conflict_resolution = "skip".into();
        context.config.settings.transfer_follow_symlinks = true;

        assert!(handle(&mut state, &mut context, TransferPromptOp::Move));
        let jobs = state.transfer.as_ref().unwrap().engine.queue.get_all();
        let job = jobs.first().expect("move job queued");
        assert_eq!(job.operation, TransferOperation::Move);
        assert!(job.options.preserve_attributes);
        assert!(job.options.follow_symlinks);
        assert_eq!(job.options.conflict_resolution, "skip");
    }

    #[test]
    fn copy_with_confirmation_opens_the_dialog() {
        let mut state = AppState::new("src".into(), "dst".into());
        state
            .get_active_panel_mut()
            .selected_paths
            .insert(std::path::PathBuf::from("src").join("a.txt"));
        let mut context = context();
        context.config.settings.confirmations.confirm_copy = true;

        assert!(handle(&mut state, &mut context, TransferPromptOp::Copy));
        match state.dialogs.top() {
            Some(PopupType::TransferPrompt(p)) => {
                assert_eq!(p.op, TransferPromptOp::Copy);
                assert_eq!(p.src_paths.len(), 1);
            }
            other => panic!("expected transfer prompt, got {other:?}"),
        }
    }
}
