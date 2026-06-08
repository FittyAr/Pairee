use crate::fs::ProgressUpdate;
use std::path::PathBuf;
use tokio::sync::mpsc;

/// Spawns a background Tokio task that applies `cmd_template` to each path in `targets`.
///
/// The template may contain `%f` which will be replaced by the quoted absolute path.
/// Progress updates are sent through the returned channel.
///
/// # Example
/// ```
/// let rx = apply_command("echo %f", vec![PathBuf::from("/tmp/a.txt")]);
/// ```
pub fn apply_command(cmd_template: String, targets: Vec<PathBuf>) -> mpsc::Receiver<ProgressUpdate> {
    let (tx, rx) = mpsc::channel(64);

    tokio::spawn(async move {
        let total = targets.len();
        for (idx, path) in targets.iter().enumerate() {
            let path_str = path.to_string_lossy();
            // Replace %f with the quoted path
            let cmd = cmd_template.replace("%f", &format!("\"{}\"", path_str));

            // Notify UI of current file
            let _ = tx
                .send(ProgressUpdate {
                    current_file: path
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_else(|| path_str.into_owned()),
                    files_copied: idx,
                    total_files: total,
                    bytes_copied: 0,
                    total_bytes: 0,
                    error: None,
                })
                .await;

            // Execute the command using the platform shell
            let result = run_shell_command(&cmd).await;

            if let Err(e) = result {
                let _ = tx
                    .send(ProgressUpdate {
                        current_file: "Completed".to_string(),
                        files_copied: idx,
                        total_files: total,
                        bytes_copied: 0,
                        total_bytes: 0,
                        error: Some(format!("Command failed for {:?}: {}", path, e)),
                    })
                    .await;
                return;
            }
        }

        // Signal completion
        let _ = tx
            .send(ProgressUpdate {
                current_file: "Completed".to_string(),
                files_copied: total,
                total_files: total,
                bytes_copied: 0,
                total_bytes: 0,
                error: None,
            })
            .await;
    });

    rx
}

/// Runs a shell command string asynchronously.
async fn run_shell_command(cmd: &str) -> anyhow::Result<()> {
    #[cfg(unix)]
    let output = tokio::process::Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .output()
        .await?;

    #[cfg(windows)]
    let output = tokio::process::Command::new("cmd")
        .arg("/C")
        .arg(cmd)
        .output()
        .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        anyhow::bail!("{}", stderr.trim());
    }
    Ok(())
}
