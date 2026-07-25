use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType, GitConfirmedAction};
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Refreshes the Git panel data and maintains a safe cursor index.
fn refresh_git_panel(state: &mut AppState, repo_path: &std::path::Path, active_tab: usize, cursor_idx: usize) {
    if let Some(mut repo) = crate::git::repo::find_repo(repo_path) {
        let new_branch = repo
            .head()
            .ok()
            .and_then(|h| h.shorthand().ok().map(|s| s.to_string()))
            .unwrap_or_else(|| "(detached HEAD)".to_string());

        let status_entries = crate::git::status::get_status(&repo);
        let log_entries = crate::git::log::get_log(&repo, 100);
        let branch_entries = crate::git::branches::get_branches(&repo);
        let stash_entries = crate::git::stash::list_stashes(&mut repo).unwrap_or_default();

        let list_len = match active_tab {
            0 => status_entries.len(),
            1 => log_entries.len(),
            2 => branch_entries.len(),
            3 => stash_entries.len(),
            _ => 0,
        };
        let safe_cursor = cursor_idx.min(list_len.saturating_sub(1));

        state.active_popup = Some(PopupType::GitPanel {
            repo_path: repo_path.to_path_buf(),
            active_tab,
            cursor_idx: safe_cursor,
            scroll: 0,
            status_entries,
            log_entries,
            branch_entries,
            stash_entries,
            current_branch: new_branch,
            pending_action: None,
        });
    }
}

/// Handles keyboard input for the main Git panel popup.
pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    _context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    if let Some(PopupType::GitPanel {
        mut active_tab,
        mut cursor_idx,
        mut scroll,
        status_entries,
        log_entries,
        branch_entries,
        stash_entries,
        repo_path,
        current_branch,
        ..
    }) = state.active_popup.clone()
    {
        let is_shift = key.modifiers.contains(KeyModifiers::SHIFT);

        let current_list_len = match active_tab {
            0 => status_entries.len(),
            1 => log_entries.len(),
            2 => branch_entries.len(),
            3 => stash_entries.len(),
            _ => 0,
        };

        match key.code {
            // ── Tab navigation (4 tabs now) ──────────────────────────────────
            KeyCode::Tab if !is_shift => {
                active_tab = (active_tab + 1) % 4;
                cursor_idx = 0;
                scroll = 0;
            }
            KeyCode::BackTab | KeyCode::Tab if is_shift => {
                active_tab = if active_tab == 0 { 3 } else { active_tab - 1 };
                cursor_idx = 0;
                scroll = 0;
            }

            // ── Cursor movement ──────────────────────────────────────────────
            KeyCode::Up => {
                if cursor_idx > 0 {
                    cursor_idx -= 1;
                }
            }
            KeyCode::Down => {
                if current_list_len > 0 && cursor_idx < current_list_len - 1 {
                    cursor_idx += 1;
                }
            }
            KeyCode::PageUp => {
                cursor_idx = cursor_idx.saturating_sub(10);
            }
            KeyCode::PageDown => {
                cursor_idx = (cursor_idx + 10).min(current_list_len.saturating_sub(1));
            }
            KeyCode::Home => {
                cursor_idx = 0;
            }
            KeyCode::End => {
                cursor_idx = current_list_len.saturating_sub(1);
            }

            // ── Global Remote Actions (Fetch / Pull / Push) ──────────────────
            KeyCode::Char('f') | KeyCode::Char('F') => {
                if let Some(repo) = crate::git::repo::find_repo(&repo_path) {
                    match crate::git::remote::fetch(&repo, "origin") {
                        Ok(_) => {
                            state.active_popup = Some(PopupType::Info(crate::config::localization::t("git_operation_success")));
                        }
                        Err(e) => {
                            state.active_popup = Some(PopupType::Error(format!("Fetch failed: {}", e)));
                        }
                    }
                }
                return Ok(None);
            }
            KeyCode::Char('l') | KeyCode::Char('L') => {
                if let Some(repo) = crate::git::repo::find_repo(&repo_path) {
                    let current_branch_name = repo.head().ok().and_then(|h| h.shorthand().ok().map(|s| s.to_string())).unwrap_or_else(|| "main".to_string());
                    match crate::git::remote::pull(&repo, "origin", &current_branch_name) {
                        Ok(_) => {
                            refresh_git_panel(state, &repo_path, active_tab, cursor_idx);
                            state.active_popup = Some(PopupType::Info(crate::config::localization::t("git_operation_success")));
                        }
                        Err(e) => {
                            state.active_popup = Some(PopupType::Error(format!("Pull failed: {}", e)));
                        }
                    }
                }
                return Ok(None);
            }
            KeyCode::Char('u') | KeyCode::Char('U') => {
                if let Some(repo) = crate::git::repo::find_repo(&repo_path) {
                    let current_branch_name = repo.head().ok().and_then(|h| h.shorthand().ok().map(|s| s.to_string())).unwrap_or_else(|| "main".to_string());
                    match crate::git::remote::push(&repo, "origin", &current_branch_name) {
                        Ok(_) => {
                            state.active_popup = Some(PopupType::Info(crate::config::localization::t("git_operation_success")));
                        }
                        Err(e) => {
                            state.active_popup = Some(PopupType::Error(format!("Push failed: {}", e)));
                        }
                    }
                }
                return Ok(None);
            }

            // ── Tab 0 (Status) Actions ───────────────────────────────────────
            KeyCode::Char(' ') if active_tab == 0 => {
                if let Some(entry) = status_entries.get(cursor_idx) {
                    if let Some(repo) = crate::git::repo::find_repo(&repo_path) {
                        let res = match entry.kind {
                            crate::git::status::StatusKind::Added => {
                                crate::git::stage::unstage_file(&repo, &entry.path)
                            }
                            _ => {
                                crate::git::stage::stage_file(&repo, &entry.path)
                            }
                        };
                        if res.is_ok() {
                            refresh_git_panel(state, &repo_path, active_tab, cursor_idx);
                        }
                    }
                }
                return Ok(None);
            }
            KeyCode::Char('c') | KeyCode::Char('C') if active_tab == 0 => {
                state.active_popup = Some(PopupType::GitCommitPrompt {
                    input: String::new(),
                    cursor_idx: 0,
                    repo_path,
                });
                return Ok(None);
            }
            KeyCode::Char('d') | KeyCode::Char('D') if active_tab == 0 => {
                if let Some(entry) = status_entries.get(cursor_idx) {
                    if let Some(repo) = crate::git::repo::find_repo(&repo_path) {
                        let is_staged = matches!(entry.kind, crate::git::status::StatusKind::Added);
                        if let Ok(diff_content) = crate::git::diff::get_file_diff(&repo, &entry.path, is_staged) {
                            let current_popup = state.active_popup.clone().unwrap();
                            state.active_popup = Some(PopupType::GitDiffView {
                                repo_path: repo_path.clone(),
                                file_path: Some(entry.path.clone()),
                                commit_hash: None,
                                diff_content,
                                scroll_y: 0,
                                previous_popup: Box::new(current_popup),
                            });
                        }
                    }
                }
                return Ok(None);
            }
            KeyCode::Char('s') | KeyCode::Char('S') if active_tab == 0 => {
                let current_popup = state.active_popup.clone().unwrap();
                state.active_popup = Some(PopupType::GitStashSavePrompt {
                    input: String::new(),
                    cursor_idx: 0,
                    repo_path: repo_path.clone(),
                    previous_popup: Box::new(current_popup),
                });
                return Ok(None);
            }

            // ── Tab 1 (Log) Actions ──────────────────────────────────────────
            KeyCode::Char('d') | KeyCode::Char('D') if active_tab == 1 => {
                if let Some(commit) = log_entries.get(cursor_idx) {
                    if let Some(repo) = crate::git::repo::find_repo(&repo_path) {
                        if let Ok(diff_content) = crate::git::diff::get_commit_diff(&repo, &commit.hash_full) {
                            let current_popup = state.active_popup.clone().unwrap();
                            state.active_popup = Some(PopupType::GitDiffView {
                                repo_path: repo_path.clone(),
                                file_path: None,
                                commit_hash: Some(commit.hash_short.clone()),
                                diff_content,
                                scroll_y: 0,
                                previous_popup: Box::new(current_popup),
                            });
                        }
                    }
                }
                return Ok(None);
            }
            KeyCode::Char('s') if active_tab == 1 => {
                if let Some(commit) = log_entries.get(cursor_idx) {
                    let current_popup = state.active_popup.clone().unwrap();
                    let msg = crate::config::localization::t("git_confirm_reset")
                        .replace("{}", &commit.hash_short)
                        .replace("{}", "Soft");
                    state.active_popup = Some(PopupType::GitConfirmAction {
                        message: msg,
                        repo_path: repo_path.clone(),
                        action: GitConfirmedAction::ResetCommit(commit.hash_full.clone(), crate::git::reset::ResetMode::Soft),
                        previous_popup: Box::new(current_popup),
                    });
                }
                return Ok(None);
            }
            KeyCode::Char('x') if active_tab == 1 => {
                if let Some(commit) = log_entries.get(cursor_idx) {
                    let current_popup = state.active_popup.clone().unwrap();
                    let msg = crate::config::localization::t("git_confirm_reset")
                        .replace("{}", &commit.hash_short)
                        .replace("{}", "Mixed");
                    state.active_popup = Some(PopupType::GitConfirmAction {
                        message: msg,
                        repo_path: repo_path.clone(),
                        action: GitConfirmedAction::ResetCommit(commit.hash_full.clone(), crate::git::reset::ResetMode::Mixed),
                        previous_popup: Box::new(current_popup),
                    });
                }
                return Ok(None);
            }
            KeyCode::Char('h') if active_tab == 1 => {
                if let Some(commit) = log_entries.get(cursor_idx) {
                    let current_popup = state.active_popup.clone().unwrap();
                    let msg = crate::config::localization::t("git_confirm_reset")
                        .replace("{}", &commit.hash_short)
                        .replace("{}", "Hard");
                    state.active_popup = Some(PopupType::GitConfirmAction {
                        message: msg,
                        repo_path: repo_path.clone(),
                        action: GitConfirmedAction::ResetCommit(commit.hash_full.clone(), crate::git::reset::ResetMode::Hard),
                        previous_popup: Box::new(current_popup),
                    });
                }
                return Ok(None);
            }
            KeyCode::Enter if active_tab == 1 => {
                if let Some(commit) = log_entries.get(cursor_idx) {
                    state.active_popup = Some(PopupType::GitConfirmCheckout {
                        target: commit.hash_full.clone(),
                        is_branch: false,
                        repo_path,
                    });
                    return Ok(None);
                }
            }

            // ── Tab 2 (Branches) Actions ─────────────────────────────────────
            KeyCode::Char('n') | KeyCode::Char('N') if active_tab == 2 => {
                let current_popup = state.active_popup.clone().unwrap();
                state.active_popup = Some(PopupType::GitBranchCreatePrompt {
                    input: String::new(),
                    cursor_idx: 0,
                    repo_path: repo_path.clone(),
                    previous_popup: Box::new(current_popup),
                });
                return Ok(None);
            }
            KeyCode::Char('d') | KeyCode::Char('D') | KeyCode::Delete if active_tab == 2 => {
                if let Some(branch) = branch_entries.get(cursor_idx) {
                    if !branch.is_current {
                        let current_popup = state.active_popup.clone().unwrap();
                        let msg = crate::config::localization::t("git_confirm_delete_branch")
                            .replace("{}", &branch.name);
                        state.active_popup = Some(PopupType::GitConfirmAction {
                            message: msg,
                            repo_path: repo_path.clone(),
                            action: GitConfirmedAction::DeleteBranch(branch.name.clone()),
                            previous_popup: Box::new(current_popup),
                        });
                    }
                }
                return Ok(None);
            }
            KeyCode::Char('r') | KeyCode::Char('R') if active_tab == 2 => {
                if let Some(branch) = branch_entries.get(cursor_idx) {
                    if !branch.is_remote {
                        let current_popup = state.active_popup.clone().unwrap();
                        state.active_popup = Some(PopupType::GitBranchRenamePrompt {
                            input: branch.name.clone(),
                            cursor_idx: branch.name.len(),
                            old_name: branch.name.clone(),
                            repo_path: repo_path.clone(),
                            previous_popup: Box::new(current_popup),
                        });
                    }
                }
                return Ok(None);
            }
            KeyCode::Char('m') | KeyCode::Char('M') if active_tab == 2 => {
                if let Some(branch) = branch_entries.get(cursor_idx) {
                    if !branch.is_current && !branch.is_remote {
                        let current_popup = state.active_popup.clone().unwrap();
                        let msg = crate::config::localization::t("git_confirm_merge_branch")
                            .replace("{}", &branch.name)
                            .replace("{}", &current_branch);
                        state.active_popup = Some(PopupType::GitConfirmAction {
                            message: msg,
                            repo_path: repo_path.clone(),
                            action: GitConfirmedAction::MergeBranch(branch.name.clone()),
                            previous_popup: Box::new(current_popup),
                        });
                    }
                }
                return Ok(None);
            }
            KeyCode::Enter if active_tab == 2 => {
                if let Some(branch) = branch_entries.get(cursor_idx) {
                    if !branch.is_remote {
                        state.active_popup = Some(PopupType::GitConfirmCheckout {
                            target: branch.name.clone(),
                            is_branch: true,
                            repo_path,
                        });
                        return Ok(None);
                    }
                }
            }

            // ── Tab 3 (Stash) Actions ────────────────────────────────────────
            KeyCode::Char('a') | KeyCode::Char('A') if active_tab == 3 => {
                if let Some(stash) = stash_entries.get(cursor_idx) {
                    if let Some(mut repo) = crate::git::repo::find_repo(&repo_path) {
                        if crate::git::stash::stash_apply(&mut repo, stash.index).is_ok() {
                            refresh_git_panel(state, &repo_path, active_tab, cursor_idx);
                            state.active_popup = Some(PopupType::Info(crate::config::localization::t("git_operation_success")));
                        }
                    }
                }
                return Ok(None);
            }
            KeyCode::Char('p') | KeyCode::Char('P') | KeyCode::Enter if active_tab == 3 => {
                if let Some(stash) = stash_entries.get(cursor_idx) {
                    let current_popup = state.active_popup.clone().unwrap();
                    let msg = crate::config::localization::t("git_confirm_stash_pop")
                        .replace("{}", &stash.index.to_string());
                    state.active_popup = Some(PopupType::GitConfirmAction {
                        message: msg,
                        repo_path: repo_path.clone(),
                        action: GitConfirmedAction::StashPop(stash.index),
                        previous_popup: Box::new(current_popup),
                    });
                }
                return Ok(None);
            }
            KeyCode::Char('d') | KeyCode::Char('D') | KeyCode::Delete if active_tab == 3 => {
                if let Some(stash) = stash_entries.get(cursor_idx) {
                    let current_popup = state.active_popup.clone().unwrap();
                    let msg = crate::config::localization::t("git_confirm_stash_drop")
                        .replace("{}", &stash.index.to_string());
                    state.active_popup = Some(PopupType::GitConfirmAction {
                        message: msg,
                        repo_path: repo_path.clone(),
                        action: GitConfirmedAction::StashDrop(stash.index),
                        previous_popup: Box::new(current_popup),
                    });
                }
                return Ok(None);
            }

            // ── Refresh ──────────────────────────────────────────────────────
            KeyCode::Char('r') | KeyCode::Char('R') | KeyCode::F(5) => {
                refresh_git_panel(state, &repo_path, active_tab, cursor_idx);
                return Ok(None);
            }

            // ── Close ────────────────────────────────────────────────────────
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
                state.active_popup = None;
                return Ok(None);
            }

            _ => return Ok(None),
        }

        // Update scroll so cursor stays in view
        if cursor_idx < scroll {
            scroll = cursor_idx;
        }

        state.active_popup = Some(PopupType::GitPanel {
            repo_path,
            active_tab,
            cursor_idx,
            scroll,
            status_entries,
            log_entries,
            branch_entries,
            stash_entries,
            current_branch,
            pending_action: None,
        });
        Ok(None)
    } else {
        Err(())
    }
}
