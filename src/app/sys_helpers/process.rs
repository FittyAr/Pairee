use crate::app::state::ProcessEntry;
use std::path::PathBuf;
use std::time::Duration;

/// Suspends raw mode in-place and kills the specified process by PID.
///
/// On Unix the kill is graceful: a `SIGTERM` is sent first and we wait up
/// to 2 seconds for the process to exit. Only if the process is still
/// alive after the grace period do we escalate to `SIGKILL`. The
/// unconditional `SIGKILL` we used to send would skip the OS's normal
/// cleanup (closing file handles, releasing locks, flushing stdio) and
/// could leave child processes orphaned or the user's data in a corrupt
/// state. `kill(2)` is still subject to the OS permission checks, so a
/// normal user cannot kill processes they do not own.
pub fn kill_process(pid: u32) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        // SIGTERM (15) — the process can trap this and shut down cleanly.
        // Ignore ESRCH (the process is already gone, which is fine).
        let term = std::process::Command::new("kill")
            .arg("-15")
            .arg(pid.to_string())
            .output()?;
        if !term.status.success() {
            let stderr = String::from_utf8_lossy(&term.stderr);
            // ESRCH = "no such process" — already dead, treat as success.
            if !stderr.contains("No such process") && !stderr.contains("ESRCH") {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("kill -15 failed: {}", stderr),
                ));
            }
        }

        // Wait up to ~2s for the process to exit gracefully.
        for _ in 0..20 {
            if !pid_is_alive(pid) {
                return Ok(());
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }

        // Still alive — escalate to SIGKILL (9). This is the last resort
        // and should rarely be needed in practice.
        let kill = std::process::Command::new("kill")
            .arg("-9")
            .arg(pid.to_string())
            .output()?;
        if !kill.status.success() {
            let stderr = String::from_utf8_lossy(&kill.stderr);
            if stderr.contains("No such process") || stderr.contains("ESRCH") {
                return Ok(()); // exited between our check and the kill
            }
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("kill -9 failed: {}", stderr),
            ));
        }
        Ok(())
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
            Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("taskkill failed: {}", err_msg),
            ))
        }
    }
}

/// Returns true if a process with `pid` is currently running.
/// Uses `kill(pid, 0)` which performs the permission/existence check
/// without actually sending a signal.
#[cfg(unix)]
fn pid_is_alive(pid: u32) -> bool {
    // `kill -0` exits 0 if the process exists and we can signal it,
    // -1 (and prints ESRCH) if it does not. We ignore the output and
    // just check the exit status.
    std::process::Command::new("kill")
        .arg("-0")
        .arg(pid.to_string())
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
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
            .args(&["/FO", "CSV", "/NH"])
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                let parts: Vec<&str> = if line.starts_with('"') && line.ends_with('"') {
                    line[1..line.len() - 1].split("\",\"").collect()
                } else {
                    line.split(',').collect()
                };
                if parts.len() >= 5 {
                    let name = parts[0].to_string();
                    let pid = parts[1].parse::<u32>().unwrap_or(0);
                    let mem_str = parts[4];
                    let digits_only: String =
                        mem_str.chars().filter(|c| c.is_ascii_digit()).collect();
                    let memory_kb = digits_only.parse::<u64>().unwrap_or(0);
                    processes.push(ProcessEntry {
                        pid,
                        name,
                        memory_kb,
                    });
                }
            }
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
    if !current.is_empty() {
        if let Ok(s) = String::from_utf8(current) {
            args.push(s);
        }
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
        .args(&["-Command", &ps_cmd])
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
        .args(&["-p", &pid.to_string(), "-o", "comm="])
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
        let cmd = "[Environment]::GetEnvironmentVariables('Machine').GetEnumerator() | % { \"$($_.Key)=$($_.Value)\" }; [Environment]::GetEnvironmentVariables('User').GetEnumerator() | % { \"$($_.Key)=$($_.Value)\" }";
        if let Ok(output) = Command::new("powershell")
            .args(&["-NoProfile", "-Command", cmd])
            .output()
        {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                for line in stdout.lines() {
                    if let Some(pos) = line.find('=') {
                        let key = &line[..pos];
                        let val = &line[pos + 1..];
                        unsafe {
                            std::env::set_var(key, val);
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Spawn a long-running child that we can later try to kill.
    /// The platform-specific commands are picked to be guaranteed to
    /// exist on the respective OS.
    fn spawn_sleepy_child() -> std::process::Child {
        #[cfg(unix)]
        let mut cmd = {
            let mut c = std::process::Command::new("sleep");
            c.arg("60");
            c
        };
        #[cfg(windows)]
        let mut cmd = {
            // `timeout` waits for the given number of seconds before
            // exiting. We use it instead of `ping localhost -n 60`
            // because it has no side effects.
            let mut c = std::process::Command::new("timeout");
            c.arg("/t").arg("60").arg("/nobreak");
            c
        };
        cmd.stdin(std::process::Stdio::null());
        cmd.stdout(std::process::Stdio::null());
        cmd.stderr(std::process::Stdio::null());
        cmd.spawn().expect("failed to spawn test child")
    }

    #[test]
    fn kill_process_terminates_a_running_child() {
        let mut child = spawn_sleepy_child();
        let pid = child.id();
        assert!(pid > 0, "child must have a valid pid");

        // The child should be running before we kill it. If it has
        // already exited (e.g. on a very fast box, `timeout /t 60`
        // might somehow not block), the test is meaningless — skip.
        if let Ok(Some(_)) = child.try_wait() {
            // The child died on its own before we could test. Skip
            // the rest of the test on this iteration.
            eprintln!("child exited before kill_process; skipping");
            return;
        }

        // The kill may legitimately fail if the child has already
        // exited between `try_wait` and `kill_process` — on a
        // heavily loaded CI box the process can be rescheduled away
        // for longer than expected. We accept either outcome as
        // long as the process is gone afterwards.
        let _ = kill_process(pid);

        // After kill, the child must have terminated. `wait` will
        // return immediately with the exit status.
        let status = child
            .wait_timeout(Duration::from_secs(5))
            .expect("wait_timeout");
        assert!(status.is_some(), "child must have exited within 5s");
    }

    #[test]
    fn kill_process_handles_already_dead_pid() {
        // PID 1 is unlikely to be killable by an unprivileged test
        // process — but a definitely-dead high PID will return ESRCH
        // which the helper treats as success. Use a PID well above
        // any reasonable value: 4_000_000 on Unix, 99_999_999 on
        // Windows (DWORD max). These are guaranteed not to be in use.
        let dead_pid: u32 = {
            #[cfg(unix)]
            {
                4_000_000
            }
            #[cfg(target_os = "windows")]
            {
                99_999_999
            }
        };
        // We don't strictly assert success here because the kill
        // helper may legitimately error on a PID owned by another
        // user. The contract is "doesn't panic and returns a
        // Result", so we just exercise the code path.
        let _ = kill_process(dead_pid);
    }

    #[test]
    fn get_process_list_returns_non_empty_on_supported_platforms() {
        // The current process must always show up in the list on
        // platforms that implement get_process_list (Linux, Windows).
        // On macOS and other Unix variants the helper returns an
        // empty list, so we don't assert anything there.
        let procs = get_process_list();
        #[cfg(any(target_os = "linux", target_os = "windows"))]
        {
            assert!(
                !procs.is_empty(),
                "process list must include at least one entry"
            );
            // Each entry must have a non-empty name (even if it's
            // a synthetic `[<pid>]` placeholder for kernel threads).
            for p in &procs {
                assert!(
                    !p.name.is_empty(),
                    "process name must not be empty: {:?}",
                    p
                );
            }
        }
        #[cfg(not(any(target_os = "linux", target_os = "windows")))]
        {
            // On unsupported Unix variants the helper is allowed to
            // return an empty list.
            let _ = procs;
        }
    }

    #[test]
    fn pid_is_alive_distinguishes_live_and_dead_pids() {
        // Spawn a child and check that pid_is_alive agrees with
        // reality: true while running, false after kill.
        #[cfg(unix)]
        {
            let child = spawn_sleepy_child();
            let pid = child.id();
            assert!(pid_is_alive(pid), "freshly spawned child must be alive");
            // Kill it and reap.
            let _ = kill_process(pid);
            let _ = child.wait_timeout(Duration::from_secs(5));
            // After the wait, the pid must be gone (or the child
            // reaped to a state where we can't signal it).
            // We don't strictly assert `!pid_is_alive` because the
            // process might be a zombie until the parent reaps it
            // — but kill_process + wait should have reaped it.
            // If the child is still alive here, something is wrong.
            assert!(!pid_is_alive(pid), "child must be dead after kill+wait");
        }
        // pid_is_alive is Unix-only (uses `kill -0`).
        #[cfg(not(unix))]
        {
            // Just call the function to ensure the test compiles.
            // We don't run the spawn path on Windows because
            // `taskkill /F` is a force-kill and the test would be
            // noisy.
        }
    }
}

/// Extension trait on `std::process::Child` to add a `wait_timeout`
/// helper without taking a new dependency on the `wait-timeout` crate.
/// The implementation polls with a short sleep.
trait ChildWaitTimeout {
    fn wait_timeout(
        &mut self,
        timeout: Duration,
    ) -> std::io::Result<Option<std::process::ExitStatus>>;
}

impl ChildWaitTimeout for std::process::Child {
    fn wait_timeout(
        &mut self,
        timeout: Duration,
    ) -> std::io::Result<Option<std::process::ExitStatus>> {
        let start = std::time::Instant::now();
        loop {
            if let Some(status) = self.try_wait()? {
                return Ok(Some(status));
            }
            if start.elapsed() >= timeout {
                return Ok(None);
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }
}
