use crate::fs::ProgressUpdate;
use std::path::PathBuf;
use tokio::sync::mpsc;

/// Spawns a background Tokio task that applies `cmd_template` to each path in `targets`.
///
/// The template is split on whitespace into a program + argv. Each argv token
/// that contains the literal substring `%f` has it replaced by the absolute
/// path of the current target. The program is then executed directly via
/// `execvp` semantics (no shell), so paths containing spaces, quotes,
/// backticks, `$()`, or other shell metacharacters cannot inject commands.
///
/// Progress updates are sent through the returned channel.
///
/// # Example
/// ```
/// let rx = apply_command("echo %f", vec![PathBuf::from("/tmp/a.txt")]);
/// ```
pub fn apply_command(
    cmd_template: String,
    targets: Vec<PathBuf>,
) -> mpsc::Receiver<ProgressUpdate> {
    let (tx, rx) = mpsc::channel(64);

    // Pre-parse the template once. This fails fast (and surfaces the error to
    // the UI before any work starts) if the user gave us a blank template.
    let parsed = match parse_command_template(&cmd_template) {
        Ok(p) => p,
        Err(e) => {
            let (tx_err, rx_err) = mpsc::channel(1);
            let _ = tx_err.try_send(ProgressUpdate {
                current_file: "Error".to_string(),
                files_copied: 0,
                total_files: 0,
                bytes_copied: 0,
                total_bytes: 0,
                error: Some(format!("Invalid command template: {}", e)),
            });
            return rx_err;
        }
    };

    tokio::spawn(async move {
        let total = targets.len();
        for (idx, path) in targets.iter().enumerate() {
            let path_str = path.to_string_lossy().to_string();

            // Notify UI of current file
            let _ = tx
                .send(ProgressUpdate {
                    current_file: path
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_else(|| path_str.clone()),
                    files_copied: idx,
                    total_files: total,
                    bytes_copied: 0,
                    total_bytes: 0,
                    error: None,
                })
                .await;

            // Build the argv for this target. Each %f in the template's
            // args is replaced with the absolute path verbatim — no quoting
            // needed because execvp does not parse a shell string.
            let argv: Vec<String> = parsed
                .args
                .iter()
                .map(|t| t.replace("%f", &path_str))
                .collect();

            // Execute the program directly (no shell). This is the fix for
            // the shell-injection bug: even if `path_str` is `evil"; rm -rf ~`,
            // it is delivered as a single argv element and the program sees
            // it as a literal filename.
            let result = run_program(&parsed.program, &argv).await;

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

/// Pre-parsed command template: program + argv tokens. Each token is a
/// literal string; `%f` substitution happens per-target at exec time.
struct ParsedTemplate {
    program: String,
    args: Vec<String>,
}

/// Parse a command template into a program + argv. Splits on ASCII
/// whitespace, treats single/double quotes as ordinary characters (we are
/// not reimplementing a shell). Returns an error if the template is empty
/// or only whitespace.
fn parse_command_template(template: &str) -> anyhow::Result<ParsedTemplate> {
    let tokens: Vec<&str> = template.split_whitespace().collect();
    if tokens.is_empty() {
        anyhow::bail!("command template is empty");
    }
    let program = tokens[0].to_string();
    let args = tokens[1..].iter().map(|s| s.to_string()).collect();
    Ok(ParsedTemplate { program, args })
}

/// Runs a program directly (no shell) and returns an error if it exits non-zero.
async fn run_program(program: &str, args: &[String]) -> anyhow::Result<()> {
    let output = tokio::process::Command::new(program)
        .args(args)
        .output()
        .await
        .map_err(|e| anyhow::anyhow!("failed to spawn `{}`: {}", program, e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        // Prefer stderr; fall back to stdout if stderr is empty.
        let detail = if !stderr.trim().is_empty() {
            stderr.trim().to_string()
        } else if !stdout.trim().is_empty() {
            stdout.trim().to_string()
        } else {
            format!("exit status {}", output.status)
        };
        anyhow::bail!("{}", detail);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_template_splits_on_whitespace() {
        let p = parse_command_template("cp %f /tmp/dest").unwrap();
        assert_eq!(p.program, "cp");
        assert_eq!(p.args, vec!["%f", "/tmp/dest"]);
    }

    #[test]
    fn parse_template_rejects_empty() {
        assert!(parse_command_template("").is_err());
        assert!(parse_command_template("   \t  ").is_err());
    }

    #[test]
    fn parse_template_preserves_percent_f_verbatim() {
        // %f is NOT expanded at parse time; it is expanded at exec time so
        // that the substituted value is never parsed by a shell.
        let p = parse_command_template("rm %f").unwrap();
        assert_eq!(p.args, vec!["%f"]);
    }
}
