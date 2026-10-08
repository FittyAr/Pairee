//! Options form: direction, content comparison, hidden files, mask.

use crate::app::form::FormKey;
use crate::app::state::popup::sync::{OPTIONS_FORM, rows};
use crate::app::state::{AppState, PopupType};
use crate::app::sync::{ScanPurpose, start_scan};
use crossterm::event::KeyEvent;

pub(super) fn handle(state: &mut AppState, key: KeyEvent) {
    let Some(PopupType::SyncDirs(dialog)) = state.dialogs.top_mut() else {
        return;
    };
    let field = (dialog.focus == rows::MASK).then_some(&mut dialog.mask);
    match OPTIONS_FORM.handle(&mut dialog.focus, field, &key) {
        FormKey::Toggle(rows::DIRECTION) | FormKey::Activate(rows::DIRECTION) => {
            dialog.cycle_direction();
        }
        FormKey::Toggle(rows::CONTENT) | FormKey::Activate(rows::CONTENT) => {
            dialog.toggle_content();
        }
        FormKey::Toggle(rows::HIDDEN) | FormKey::Activate(rows::HIDDEN) => {
            dialog.options.ignore_hidden = !dialog.options.ignore_hidden;
        }
        FormKey::Activate(rows::MASK | rows::COMPARE_BUTTON) => {
            dialog.options = dialog.run_options();
            let (left, right) = (dialog.left.clone(), dialog.right.clone());
            let options = dialog.options.clone();
            start_scan(state, ScanPurpose::Sync, left, right, options);
        }
        FormKey::Activate(_) | FormKey::Cancel => {
            state.dialogs.pop();
        }
        FormKey::Toggle(_) | FormKey::Handled | FormKey::Other => {}
    }
}
