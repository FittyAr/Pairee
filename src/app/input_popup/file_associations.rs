use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType};
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent};

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    _context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    if let Some(PopupType::FileAssociationsDialog {
        mut rules,
        mut cursor_idx,
        mut editing_idx,
        mut editing_field,
        mut edit_buffer,
        mut original_rule,
    }) = state.active_popup.clone()
    {
        if let Some(idx) = editing_idx {
            // Modo Edición
            match key.code {
                KeyCode::Esc => {
                    // Cancelar edición
                    if let Some(orig) = original_rule {
                        if idx < rules.len() {
                            rules[idx] = orig;
                        }
                    } else {
                        // Era una regla nueva, remover de la lista
                        if idx < rules.len() {
                            rules.remove(idx);
                        }
                    }
                    editing_idx = None;
                    original_rule = None;
                    edit_buffer.clear();
                }
                KeyCode::Enter => {
                    // Validar y avanzar
                    let val = edit_buffer.trim().to_string();
                    match editing_field {
                        0 => {
                            if !val.is_empty() {
                                rules[idx].mask = val;
                                editing_field = 1;
                                edit_buffer = rules[idx].open_cmd.clone();
                            }
                        }
                        1 => {
                            if !val.is_empty() {
                                rules[idx].open_cmd = val;
                                editing_field = 2;
                                edit_buffer = rules[idx].view_cmd.clone().unwrap_or_default();
                            }
                        }
                        2 => {
                            if val.is_empty() {
                                rules[idx].view_cmd = None;
                            } else {
                                rules[idx].view_cmd = Some(val);
                            }
                            editing_idx = None;
                            original_rule = None;
                            edit_buffer.clear();

                            // Guardar cambios
                            let config = crate::config::associations::AssociationsConfig {
                                rules: rules.clone(),
                            };
                            let _ = config.save();
                        }
                        _ => {}
                    }
                }
                KeyCode::Backspace => {
                    edit_buffer.pop();
                }
                KeyCode::Char(c) => {
                    edit_buffer.push(c);
                }
                _ => {}
            }
        } else {
            // Modo Navegación / Operaciones
            match key.code {
                KeyCode::Esc => {
                    state.active_popup = None;
                    return Ok(None);
                }
                KeyCode::Up => {
                    if !rules.is_empty() {
                        cursor_idx = if cursor_idx > 0 {
                            cursor_idx - 1
                        } else {
                            rules.len() - 1
                        };
                    }
                }
                KeyCode::Down => {
                    if !rules.is_empty() {
                        cursor_idx = if cursor_idx < rules.len() - 1 {
                            cursor_idx + 1
                        } else {
                            0
                        };
                    }
                }
                KeyCode::Char('a') | KeyCode::Char('A') | KeyCode::Insert => {
                    // Añadir nueva regla
                    let new_rule = crate::config::associations::AssocRule {
                        mask: String::new(),
                        open_cmd: String::new(),
                        view_cmd: None,
                    };
                    rules.push(new_rule);
                    let new_idx = rules.len() - 1;
                    cursor_idx = new_idx;
                    editing_idx = Some(new_idx);
                    editing_field = 0;
                    edit_buffer.clear();
                    original_rule = None;
                }
                KeyCode::Char('e') | KeyCode::Char('E') | KeyCode::Enter => {
                    // Editar regla actual
                    if !rules.is_empty() && cursor_idx < rules.len() {
                        let rule = rules[cursor_idx].clone();
                        editing_idx = Some(cursor_idx);
                        editing_field = 0;
                        edit_buffer = rule.mask.clone();
                        original_rule = Some(rule);
                    }
                }
                KeyCode::Char('d') | KeyCode::Char('D') | KeyCode::Delete => {
                    // Eliminar regla actual
                    if !rules.is_empty() && cursor_idx < rules.len() {
                        rules.remove(cursor_idx);
                        if cursor_idx >= rules.len() && !rules.is_empty() {
                            cursor_idx = rules.len() - 1;
                        }
                        // Guardar cambios
                        let config = crate::config::associations::AssociationsConfig {
                            rules: rules.clone(),
                        };
                        let _ = config.save();
                    }
                }
                _ => {}
            }
        }

        state.active_popup = Some(PopupType::FileAssociationsDialog {
            rules,
            cursor_idx,
            editing_idx,
            editing_field,
            edit_buffer,
            original_rule,
        });
        Ok(None)
    } else {
        Err(())
    }
}
