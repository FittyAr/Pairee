use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType, ProcessEntry};
use crate::app::sys_helpers::{get_process_list, kill_process};
use crate::config::localization::t;
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent};

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    _context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    let Some(PopupType::TaskListDialog {
        tasks,
        cursor_idx,
        filter_query,
        is_filtering,
    }) = state.dialogs.top_mut()
    else {
        return Err(());
    };
    if *is_filtering {
        match key.code {
            KeyCode::Esc => {
                filter_query.clear();
                *is_filtering = false;
                *cursor_idx = 0;
                apply_filter(tasks, filter_query.text());
            }
            KeyCode::Enter => {
                *is_filtering = false;
                clamp_cursor(tasks, filter_query.text(), cursor_idx);
            }
            _ => {
                if filter_query.handle_edit_key(&key).consumed() {
                    apply_filter(tasks, filter_query.text());
                    clamp_cursor(tasks, filter_query.text(), cursor_idx);
                }
            }
        }
        return Ok(None);
    }
    match key.code {
        KeyCode::Esc if !filter_query.is_empty() => {
            filter_query.clear();
            *cursor_idx = 0;
            apply_filter(tasks, filter_query.text());
        }
        KeyCode::Esc => state.dialogs.clear(),
        KeyCode::Char('/') => *is_filtering = true,
        KeyCode::Up => *cursor_idx = cursor_idx.saturating_sub(1),
        KeyCode::Down => {
            if *cursor_idx + 1 < get_matching_count(tasks, filter_query.text()) {
                *cursor_idx += 1;
            }
        }
        KeyCode::Delete | KeyCode::Char('k' | 'K') => {
            on_selected(state, kill_process, "error_kill_process_failed")
        }
        KeyCode::Char('r' | 'R') => on_selected(
            state,
            crate::app::sys_helpers::restart_process,
            "error_restart_process_failed",
        ),
        _ => {}
    }
    Ok(None)
}

/// Runs `action` on the selected process, then reloads the list; shows
/// `error_key` on failure.
fn on_selected<E: std::fmt::Display>(
    state: &mut AppState,
    action: fn(u32) -> Result<(), E>,
    error_key: &str,
) {
    let Some(PopupType::TaskListDialog {
        tasks,
        cursor_idx,
        filter_query,
        ..
    }) = state.dialogs.top_mut()
    else {
        return;
    };
    let Some(pid) = tasks.get(*cursor_idx).map(|task| task.pid) else {
        return;
    };
    match action(pid) {
        Ok(()) => {
            *tasks = get_process_list();
            apply_filter(tasks, filter_query.text());
            clamp_cursor(tasks, filter_query.text(), cursor_idx);
        }
        Err(e) => state
            .dialogs
            .replace(PopupType::Error(t(error_key).replace("{}", &e.to_string()))),
    }
}

/// Keeps the cursor on a row matching the filter.
fn clamp_cursor(tasks: &[ProcessEntry], query: &str, cursor: &mut usize) {
    *cursor = (*cursor).min(get_matching_count(tasks, query).saturating_sub(1));
}

fn apply_filter(tasks: &mut Vec<ProcessEntry>, query: &str) {
    tasks.sort_by_key(|p| p.pid);
    if query.is_empty() {
        return;
    }
    let query_lower = query.to_lowercase();
    let mut matching = Vec::new();
    let mut non_matching = Vec::new();
    for task in tasks.drain(..) {
        if task.name.to_lowercase().contains(&query_lower) {
            matching.push(task);
        } else {
            non_matching.push(task);
        }
    }
    tasks.extend(matching);
    tasks.extend(non_matching);
}

fn get_matching_count(tasks: &[ProcessEntry], query: &str) -> usize {
    if query.is_empty() {
        tasks.len()
    } else {
        let query_lower = query.to_lowercase();
        tasks
            .iter()
            .filter(|t| t.name.to_lowercase().contains(&query_lower))
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_filter_empty() {
        let mut tasks = vec![
            ProcessEntry {
                pid: 10,
                name: "foo".to_string(),
                memory_kb: 100,
            },
            ProcessEntry {
                pid: 5,
                name: "bar".to_string(),
                memory_kb: 200,
            },
        ];
        apply_filter(&mut tasks, "");
        assert_eq!(tasks[0].pid, 5);
        assert_eq!(tasks[1].pid, 10);
    }

    #[test]
    fn test_apply_filter_matching() {
        let mut tasks = vec![
            ProcessEntry {
                pid: 1,
                name: "nginx".to_string(),
                memory_kb: 100,
            },
            ProcessEntry {
                pid: 2,
                name: "systemd".to_string(),
                memory_kb: 200,
            },
            ProcessEntry {
                pid: 3,
                name: "bash".to_string(),
                memory_kb: 300,
            },
            ProcessEntry {
                pid: 4,
                name: "sh".to_string(),
                memory_kb: 400,
            },
        ];
        apply_filter(&mut tasks, "sh");
        // Matches "bash" and "sh"
        // Since we stable partition, matching should be first: "bash" (pid 3) and "sh" (pid 4)
        // Non-matching next: "nginx" (pid 1) and "systemd" (pid 2)
        assert_eq!(tasks[0].name, "bash");
        assert_eq!(tasks[1].name, "sh");
        assert_eq!(tasks[2].name, "nginx");
        assert_eq!(tasks[3].name, "systemd");

        let count = get_matching_count(&tasks, "sh");
        assert_eq!(count, 2);
    }
}
