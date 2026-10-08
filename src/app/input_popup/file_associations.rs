//! File associations dialog: list of mask → open / view commands; a rule is
//! edited field by field (mask, open command, view command).

use crate::app::context::AppContext;
use crate::app::list_nav::{ListKey, ListKeys, list_key};
use crate::app::state::{AppState, PopupType};
use crate::app::text_input::TextField;
use crate::config::associations::{AssocRule, AssociationsConfig};
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent};

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    _context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    let Some(PopupType::FileAssociationsDialog {
        rules,
        cursor_idx,
        editing_idx,
        editing_field,
        edit_buffer,
        original_rule,
    }) = state.dialogs.top_mut()
    else {
        return Err(());
    };

    if let Some(idx) = *editing_idx {
        match key.code {
            KeyCode::Esc => {
                // Cancel: restore the rule, or drop a rule that was new.
                match original_rule.take() {
                    Some(orig) if idx < rules.len() => rules[idx] = orig,
                    None if idx < rules.len() => {
                        rules.remove(idx);
                    }
                    _ => {}
                }
                *editing_idx = None;
                edit_buffer.clear();
            }
            KeyCode::Enter => {
                let value = edit_buffer.text().trim().to_string();
                if let Some(next) = commit_field(&mut rules[idx], *editing_field, value) {
                    *editing_field += 1;
                    edit_buffer.set_text(next);
                } else if *editing_field == VIEW_FIELD {
                    *editing_idx = None;
                    *original_rule = None;
                    edit_buffer.clear();
                    save(rules);
                }
            }
            _ => {
                edit_buffer.handle_key(&key);
            }
        }
        return Ok(None);
    }

    match list_key(ListKeys::ARROWS, key.code, cursor_idx, rules.len()) {
        ListKey::Moved => {}
        ListKey::Close => state.dialogs.clear(),
        ListKey::Activate(_) => start_edit(
            rules,
            *cursor_idx,
            editing_idx,
            editing_field,
            edit_buffer,
            original_rule,
        ),
        ListKey::Other => match key.code {
            KeyCode::Char('a' | 'A') | KeyCode::Insert => {
                rules.push(AssocRule {
                    mask: String::new(),
                    open_cmd: String::new(),
                    view_cmd: None,
                });
                *cursor_idx = rules.len() - 1;
                *editing_idx = Some(*cursor_idx);
                *editing_field = MASK_FIELD;
                edit_buffer.clear();
                *original_rule = None;
            }
            KeyCode::Char('e' | 'E') => start_edit(
                rules,
                *cursor_idx,
                editing_idx,
                editing_field,
                edit_buffer,
                original_rule,
            ),
            KeyCode::Char('d' | 'D') | KeyCode::Delete if *cursor_idx < rules.len() => {
                rules.remove(*cursor_idx);
                *cursor_idx = (*cursor_idx).min(rules.len().saturating_sub(1));
                save(rules);
            }
            _ => {}
        },
    }
    Ok(None)
}

const MASK_FIELD: usize = 0;
const VIEW_FIELD: usize = 2;

/// Edits rule `idx` starting from its mask (keeping a copy for Esc).
fn start_edit(
    rules: &[AssocRule],
    idx: usize,
    editing_idx: &mut Option<usize>,
    editing_field: &mut usize,
    edit_buffer: &mut TextField,
    original_rule: &mut Option<AssocRule>,
) {
    if let Some(rule) = rules.get(idx) {
        *editing_idx = Some(idx);
        *editing_field = MASK_FIELD;
        edit_buffer.set_text(rule.mask.as_str());
        *original_rule = Some(rule.clone());
    }
}

/// Stores `value` in field `field` of `rule`. Returns the text of the next
/// field to edit, or `None` when the value was rejected (empty mask / open
/// command) or this was the last field.
fn commit_field(rule: &mut AssocRule, field: usize, value: String) -> Option<String> {
    match field {
        MASK_FIELD if !value.is_empty() => {
            rule.mask = value;
            Some(rule.open_cmd.clone())
        }
        1 if !value.is_empty() => {
            rule.open_cmd = value;
            Some(rule.view_cmd.clone().unwrap_or_default())
        }
        VIEW_FIELD => {
            rule.view_cmd = (!value.is_empty()).then_some(value);
            None
        }
        _ => None,
    }
}

fn save(rules: &[AssocRule]) {
    let _ = AssociationsConfig {
        rules: rules.to_vec(),
    }
    .save();
}
