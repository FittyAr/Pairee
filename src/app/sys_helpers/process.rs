use crate::app::state::ProcessEntry;
use std::path::PathBuf;

/// Suspends raw mode in-place and kills the specified process by PID.
pub fn kill_process(pid: u32) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        let output = std::process::Command::new("kill")
            .arg("-9")
            .arg(pid.to_string())
            .output()?;
        if output.status.success() {
            Ok(())
        } else {
            let err_msg = String::from_utf8_lossy(&output.stderr);
            Err(std::io::Error::other(format!("kill failed: {}", err_msg)))
        }
    }
    #[cfg(not(unix))]
    {
        let output = std::process::Command::new("taskkill")
            .arg("/F")
            .arg("/PID")
            .arg(pid.to_string())
            .output()?;
        if output.status.success() {
            Ok(())
        } else {
            let err_msg = String::from_utf8_lossy(&output.stderr);
            Err(std::io::Error::other(format!(
                "taskkill failed: {}",
                err_msg
            )))
        }
    }
}

/// Returns a list of running OS processes.
/// On Linux reads from /proc; on other platforms returns an empty list.
pub fn get_process_list() -> Vec<ProcessEntry> {
    #[allow(unused_mut)]
    let mut processes = Vec::new();

    #[cfg(target_os = "linux")]
    {
        if let Ok(read_dir) = std::fs::read_dir("/proc") {
            for entry in read_dir.flatten() {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                // /proc/<pid> directories have purely numeric names
                if let Ok(pid) = name_str.parse::<u32>() {
                    let comm_path = entry.path().join("comm");
                    let proc_name = std::fs::read_to_string(&comm_path)
                        .unwrap_or_default()
                        .trim()
                        .to_string();
                    // Read VmRSS from status for memory approximation
                    let memory_kb = read_proc_memory(pid);
                    processes.push(ProcessEntry {
                        pid,
                        name: if proc_name.is_empty() {
                            format!("[{}]", pid)
                        } else {
                            proc_name
                        },
                        memory_kb,
                    });
                }
            }
        }
        processes.sort_by_key(|p| p.pid);
    }

    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        if let Ok(output) = Command::new("tasklist")
            .args(["/FO", "CSV", "/NH"])
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            processes.extend(stdout.lines().filter_map(parse_tasklist_line));
        }
        processes.sort_by_key(|p| p.pid);
    }

    processes
}

#[cfg(target_os = "linux")]
fn read_proc_memory(pid: u32) -> u64 {
    let status_path = format!("/proc/{}/status", pid);
    if let Ok(content) = std::fs::read_to_string(&status_path) {
        for line in content.lines() {
            if line.starts_with("VmRSS:") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if let Some(kb_str) = parts.get(1) {
                    return kb_str.parse::<u64>().unwrap_or(0);
                }
            }
        }
    }
    0
}

#[cfg(target_os = "linux")]
fn get_process_restart_info(pid: u32) -> Option<(String, Vec<String>, Option<PathBuf>)> {
    let cmdline_bytes = std::fs::read(format!("/proc/{}/cmdline", pid)).ok()?;
    if cmdline_bytes.is_empty() {
        return None;
    }
    let mut args = Vec::new();
    let mut current = Vec::new();
    for &b in &cmdline_bytes {
        if b == 0 {
            if !current.is_empty() {
                if let Ok(s) = String::from_utf8(current.clone()) {
                    args.push(s);
                }
                current.clear();
            }
        } else {
            current.push(b);
        }
    }
    if !current.is_empty()
        && let Ok(s) = String::from_utf8(current)
    {
        args.push(s);
    }
    if args.is_empty() {
        return None;
    }
    let executable = args[0].clone();
    let remaining_args = args[1..].to_vec();
    let cwd = std::fs::read_link(format!("/proc/{}/cwd", pid)).ok();
    Some((executable, remaining_args, cwd))
}

#[cfg(target_os = "windows")]
fn get_process_restart_info(pid: u32) -> Option<(String, Vec<String>, Option<PathBuf>)> {
    use std::process::Command;
    let ps_cmd = format!(
        "Get-CimInstance Win32_Process -Filter 'ProcessId = {}' | ForEach-Object {{ $_.Path + '|' + $_.CommandLine + '|' + $_.WorkingDirectory }}",
        pid
    );
    let output = Command::new("powershell")
        .args(["-Command", &ps_cmd])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let line = stdout.trim();
    if line.is_empty() {
        return None;
    }
    let parts: Vec<&str> = line.split('|').collect();
    if parts.len() >= 2 {
        let _executable = parts[0].to_string();
        let cmd_line = parts[1].to_string();
        let cwd_str = parts.get(2).map(|s| s.to_string());

        let executable = "cmd".to_string();
        let args = vec!["/C".to_string(), cmd_line];
        let cwd = cwd_str.filter(|s| !s.trim().is_empty()).map(PathBuf::from);

        Some((executable, args, cwd))
    } else {
        None
    }
}

#[cfg(all(unix, not(target_os = "linux")))]
fn get_process_restart_info(pid: u32) -> Option<(String, Vec<String>, Option<PathBuf>)> {
    use std::process::Command;
    let output = Command::new("ps")
        .args(["-p", &pid.to_string(), "-o", "comm="])
        .output()
        .ok()?;
    if output.status.success() {
        let name = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !name.is_empty() {
            return Some((name, Vec::new(), None));
        }
    }
    None
}

pub fn restart_process(pid: u32) -> std::io::Result<()> {
    let info = get_process_restart_info(pid).ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Process info not found or already terminated",
        )
    })?;

    kill_process(pid)?;

    let mut cmd = std::process::Command::new(&info.0);
    cmd.args(&info.1);
    if let Some(cwd) = info.2 {
        cmd.current_dir(cwd);
    }
    cmd.spawn()?;
    Ok(())
}

/// Refreshes the current process environment variables from the registry on Windows.
pub fn refresh_env_vars() {
    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        let cmd = "foreach ($s in 'Machine','User') { [Environment]::GetEnvironmentVariables($s).GetEnumerator() | % { \"$s`t$($_.Key)=$($_.Value)\" } }";
        if let Ok(output) = Command::new("powershell")
            .args(["-NoProfile", "-Command", cmd])
            .output()
            && output.status.success()
        {
            for (key, val) in merge_env_scopes(&String::from_utf8_lossy(&output.stdout)) {
                // SAFETY: called on the UI thread before spawning a program;
                // no other thread reads the environment at that moment.
                unsafe {
                    std::env::set_var(key, val);
                }
            }
        }
    }
}

/// The variables of `listing` (`Machine<TAB>KEY=VALUE` and `User<TAB>...`
/// lines) as Windows builds a new process environment: user values win,
/// except `Path`, which is the machine one followed by the user one.
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
fn merge_env_scopes(listing: &str) -> Vec<(String, String)> {
    let mut merged: Vec<(String, String)> = Vec::new();
    for line in listing.lines() {
        let Some((scope, pair)) = line.split_once('\t') else {
            continue;
        };
        let Some((key, value)) = pair.split_once('=').filter(|(k, _)| !k.is_empty()) else {
            continue;
        };
        match merged.iter_mut().find(|(k, _)| k.eq_ignore_ascii_case(key)) {
            Some((_, existing)) if scope == "User" && key.eq_ignore_ascii_case("path") => {
                if !value.is_empty() {
                    existing.push(';');
                    existing.push_str(value);
                }
            }
            Some((_, existing)) => *existing = value.to_string(),
            None => merged.push((key.to_string(), value.to_string())),
        }
    }
    merged
}

/// A `tasklist /FO CSV /NH` row (`"name","pid","session","#","1,234 K"`).
#[cfg_attr(not(any(test, windows)), allow(dead_code))]
fn parse_tasklist_line(line: &str) -> Option<ProcessEntry> {
    let line = line.trim();
    let parts: Vec<&str> = match line
        .strip_prefix('"')
        .and_then(|rest| rest.strip_suffix('"'))
    {
        Some(inner) => inner.split("\",\"").collect(),
        None => line.split(',').collect(),
    };
    if parts.len() < 5 {
        return None;
    }
    let digits_only: String = parts[4].chars().filter(char::is_ascii_digit).collect();
    Some(ProcessEntry {
        pid: parts[1].parse::<u32>().unwrap_or(0),
        name: parts[0].to_string(),
        memory_kb: digits_only.parse::<u64>().unwrap_or(0),
    })
}

#[cfg(test)]
mod env_tests {
    use super::parse_tasklist_line;

    #[test]
    fn tasklist_rows_parse_and_a_lone_quote_does_not_panic() {
        let row = parse_tasklist_line("\"a.exe\",\"42\",\"Console\",\"1\",\"1,234 K\"").unwrap();
        assert_eq!(
            (row.pid, row.name.as_str(), row.memory_kb),
            (42, "a.exe", 1234)
        );
        for junk in ["\"", "\"\"", "", "a,b"] {
            assert!(parse_tasklist_line(junk).is_none(), "{junk:?}");
        }
    }

    use super::merge_env_scopes;

    #[test]
    fn user_path_extends_the_machine_path() {
        let listing = "Machine\tPath=C:\\Windows\nMachine\tTEMP=C:\\T\nUser\tPATH=C:\\Tools\nUser\tTEMP=C:\\U\nbroken line\n";
        let merged = merge_env_scopes(listing);
        assert_eq!(
            merged,
            [
                ("Path".to_string(), "C:\\Windows;C:\\Tools".to_string()),
                ("TEMP".to_string(), "C:\\U".to_string()),
            ]
        );
    }
}
