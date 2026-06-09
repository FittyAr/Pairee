use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType};
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent};

const MAX_CURSOR_IDX: usize = 13;

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    if let Some(PopupType::RenMovPrompt {
        input,
        src_paths,
        dest_dir,
        cursor_idx,
        already_existing,
        process_multiple,
        copy_access_mode,
        copy_extended_attributes,
        disable_write_cache,
        produce_sparse_files,
        use_copy_on_write,
        symlink_mode,
        use_filter,
        filter_mask,
    }) = state.active_popup.clone()
    {
        let mut new_input = input.clone();
        let mut new_idx = cursor_idx;
        let mut new_already = already_existing;
        let mut new_multi = process_multiple;
        let mut new_access = copy_access_mode;
        let mut new_ext = copy_extended_attributes;
        let mut new_cache = disable_write_cache;
        let mut new_sparse = produce_sparse_files;
        let mut new_cow = use_copy_on_write;
        let mut new_sym = symlink_mode;
        let mut new_filter = use_filter;
        let new_filter_mask = filter_mask.clone();

        let update_popup = |
            s: &mut AppState,
            i: String,
            idx: usize,
            a: usize,
            m: bool,
            ac: bool,
            ex: bool,
            ca: bool,
            sp: bool,
            cw: bool,
            sy: usize,
            f: bool,
            fm: String
        | {
            s.active_popup = Some(PopupType::RenMovPrompt {
                input: i,
                src_paths: src_paths.clone(),
                dest_dir: dest_dir.clone(),
                cursor_idx: idx,
                already_existing: a,
                process_multiple: m,
                copy_access_mode: ac,
                copy_extended_attributes: ex,
                disable_write_cache: ca,
                produce_sparse_files: sp,
                use_copy_on_write: cw,
                symlink_mode: sy,
                use_filter: f,
                filter_mask: fm,
            });
        };

        match key.code {
            KeyCode::Up | KeyCode::BackTab => {
                new_idx = if new_idx > 0 { new_idx - 1 } else { MAX_CURSOR_IDX };
                update_popup(state, new_input, new_idx, new_already, new_multi, new_access, new_ext, new_cache, new_sparse, new_cow, new_sym, new_filter, new_filter_mask);
                return Ok(None);
            }
            KeyCode::Down | KeyCode::Tab => {
                new_idx = if new_idx < MAX_CURSOR_IDX { new_idx + 1 } else { 0 };
                update_popup(state, new_input, new_idx, new_already, new_multi, new_access, new_ext, new_cache, new_sparse, new_cow, new_sym, new_filter, new_filter_mask);
                return Ok(None);
            }
            KeyCode::Char(c) => {
                if new_idx == 0 {
                    new_input.push(c);
                    update_popup(state, new_input, new_idx, new_already, new_multi, new_access, new_ext, new_cache, new_sparse, new_cow, new_sym, new_filter, new_filter_mask);
                } else if c == ' ' {
                    // Toggle depending on idx
                    match new_idx {
                        1 => new_already = (new_already + 1) % 4, // Cycle Ask, Overwrite, Skip, Append
                        2 => new_multi = !new_multi,
                        3 => new_access = !new_access,
                        4 => new_ext = !new_ext,
                        5 => new_cache = !new_cache,
                        6 => new_sparse = !new_sparse,
                        7 => new_cow = !new_cow,
                        8 => new_sym = (new_sym + 1) % 3, // Cycle Smart, Link, Target
                        9 => new_filter = !new_filter,
                        _ => {}
                    }
                    update_popup(state, new_input, new_idx, new_already, new_multi, new_access, new_ext, new_cache, new_sparse, new_cow, new_sym, new_filter, new_filter_mask);
                }
                return Ok(None);
            }
            KeyCode::Backspace => {
                if new_idx == 0 {
                    new_input.pop();
                    update_popup(state, new_input, new_idx, new_already, new_multi, new_access, new_ext, new_cache, new_sparse, new_cow, new_sym, new_filter, new_filter_mask);
                }
                return Ok(None);
            }
            KeyCode::Enter => {
                if new_idx == 13 {
                    // Cancel
                    state.active_popup = None;
                    return Ok(None);
                }

                if new_idx == 11 {
                    let nodes = crate::app::sys_helpers::build_tree_nodes(&dest_dir, 0, 3);
                    state.active_popup = Some(PopupType::TreeView {
                        nodes,
                        cursor_idx: 0,
                        caller: crate::app::state::types::TreeViewCaller::RenMovPrompt {
                            previous: Box::new(state.active_popup.take().unwrap()),
                        },
                    });
                    return Ok(None);
                }
                if new_idx == 12 {
                    state.active_popup = Some(PopupType::CopyMoveFilterPrompt {
                        input: new_filter_mask,
                        previous: Box::new(state.active_popup.take().unwrap()),
                    });
                    return Ok(None);
                }

                // Move logic
                let targets = src_paths.clone();
                let dest = if targets.len() == 1 {
                    dest_dir.join(&new_input)
                } else {
                    dest_dir.clone()
                };

                if context.config.settings.confirmations.confirm_overwrite {
                    let mut any_exists = false;
                    for src in &targets {
                        if let Some(fname) = src.file_name() {
                            let dst = if targets.len() == 1 { dest.clone() } else { dest.join(fname) };
                            if dst.exists() {
                                any_exists = true;
                                break;
                            }
                        }
                    }

                    if any_exists && new_already == 0 /* Ask */ {
                        state.active_popup = Some(PopupType::ConfirmOverwrite {
                            src_paths,
                            dest_dir,
                            is_move: true,
                            input: Some(new_input),
                        });
                        return Ok(None);
                    }
                }

                state.active_popup = None;
                for src in &targets {
                    if let Some(fname) = src.file_name() {
                        let dst = if targets.len() == 1 { dest.clone() } else { dest.join(fname) };
                        if let Err(e) = crate::fs::rename_or_move_sync(
                            src,
                            &dst,
                            context.config.settings.req_admin_modification,
                        ) {
                            state.active_popup =
                                Some(PopupType::Error(format!("{} {}", crate::config::localization::t("error_move_failed"), e)));
                            break;
                        }
                    }
                }
                state.get_active_panel_mut().selected_paths.clear();
                state.refresh_both_panels(context.config.settings.show_hidden);

                return Ok(None);
            }
            KeyCode::Esc => {
                state.active_popup = None;
                return Ok(None);
            }
            KeyCode::F(10) => {
                let nodes = crate::app::sys_helpers::build_tree_nodes(&dest_dir, 0, 3);
                state.active_popup = Some(PopupType::TreeView {
                    nodes,
                    cursor_idx: 0,
                    caller: crate::app::state::types::TreeViewCaller::RenMovPrompt {
                        previous: Box::new(state.active_popup.take().unwrap()),
                    },
                });
                return Ok(None);
            }
            _ => {}
        }
        Err(())
    } else {
        Err(())
    }
}
