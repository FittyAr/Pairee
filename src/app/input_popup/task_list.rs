use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType, ProcessEntry};
use crate::app::sys_helpers::{get_process_list, kill_process};
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent};

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    _context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    if let Some(PopupType::TaskListDialog {
        mut tasks,
        mut cursor_idx,
        mut filter_query,
        mut is_filtering,
    }) = state.active_popup.clone()
    {
        if is_filtering {
            match key.code {
                KeyCode::Esc => {
                    filter_query.clear();
                    is_filtering = false;
                    cursor_idx = 0;
                    apply_filter(&mut tasks, &filter_query);
                }
                KeyCode::Enter => {
                    is_filtering = false;
                    let matches = get_matching_count(&tasks, &filter_query);
                    if matches == 0 {
                        cursor_idx = 0;
                    } else if cursor_idx >= matches {
                        cursor_idx = matches.saturating_sub(1);
                    }
                }
                KeyCode::Backspace => {
                    filter_query.pop();
                    apply_filter(&mut tasks, &filter_query);
                    let matches = get_matching_count(&tasks, &filter_query);
                    if matches == 0 {
                        cursor_idx = 0;
                    } else if cursor_idx >= matches {
                        cursor_idx = matches.saturating_sub(1);
                    }
                }
                KeyCode::Char(c) => {
                    filter_query.push(c);
                    apply_filter(&mut tasks, &filter_query);
                    let matches = get_matching_count(&tasks, &filter_query);
                    if matches == 0 {
                        cursor_idx = 0;
                    } else if cursor_idx >= matches {
                        cursor_idx = matches.saturating_sub(1);
                    }
                }
                _ => {}
            }
        } else {
            match key.code {
                KeyCode::Esc => {
                    if !filter_query.is_empty() {
                        filter_query.clear();
                        cursor_idx = 0;
                        apply_filter(&mut tasks, &filter_query);
                    } else {
                        state.active_popup = None;
                        return Ok(None);
                    }
                }
                KeyCode::Char('/') => {
                    is_filtering = true;
                }
                KeyCode::Up => {
                    if cursor_idx > 0 {
                        cursor_idx -= 1;
                    }
                }
                KeyCode::Down => {
                    let limit = get_matching_count(&tasks, &filter_query);
                    if limit > 0 && cursor_idx < limit.saturating_sub(1) {
                        cursor_idx += 1;
                    }
                }
                KeyCode::Delete | KeyCode::Char('k') | KeyCode::Char('K') => {
                    if let Some(task) = tasks.get(cursor_idx) {
                        let pid = task.pid;
                        match kill_process(pid) {
                            Ok(_) => {
                                tasks = get_process_list();
                                apply_filter(&mut tasks, &filter_query);
                                let limit = get_matching_count(&tasks, &filter_query);
                                if limit == 0 {
                                    cursor_idx = 0;
                                } else if cursor_idx >= limit {
                                    cursor_idx = limit.saturating_sub(1);
                                }
                            }
                            Err(e) => {
                                state.active_popup = Some(PopupType::Error(format!(
                                    "Failed to kill process: {}",
                                    e
                                )));
                                return Ok(None);
                            }
                        }
                    }
                }
                KeyCode::Char('r') | KeyCode::Char('R') => {
                    if let Some(task) = tasks.get(cursor_idx) {
                        let pid = task.pid;
                        match crate::app::sys_helpers::restart_process(pid) {
                            Ok(_) => {
                                tasks = get_process_list();
                                apply_filter(&mut tasks, &filter_query);
                                let limit = get_matching_count(&tasks, &filter_query);
                                if limit == 0 {
                                    cursor_idx = 0;
                                } else if cursor_idx >= limit {
                                    cursor_idx = limit.saturating_sub(1);
                                }
                            }
                            Err(e) => {
                                state.active_popup = Some(PopupType::Error(format!(
                                    "Failed to restart process: {}",
                                    e
                                )));
                                return Ok(None);
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        state.active_popup = Some(PopupType::TaskListDialog {
            tasks,
            cursor_idx,
            filter_query,
            is_filtering,
        });
        return Ok(None);
    }
    Err(())
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
