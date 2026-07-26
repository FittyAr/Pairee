use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PostAction {
    None,
    Shutdown,
    Sleep,
    Hibernate,
    EjectDrive(String),
    RunScript(std::path::PathBuf),
    CloseApp,
}

/// Ejecuta la acción post-procesamiento correspondiente de manera multiplataforma.
pub fn execute_post_action(action: PostAction) -> Result<(), std::io::Error> {
    match action {
        PostAction::None => Ok(()),
        PostAction::Shutdown => {
            #[cfg(target_os = "windows")]
            {
                Command::new("shutdown")
                    .args([
                        "/s",
                        "/t",
                        "10",
                        "/c",
                        "Pairee: Transfer complete. Shutting down...",
                    ])
                    .spawn()?;
            }
            #[cfg(not(target_os = "windows"))]
            {
                Command::new("shutdown")
                    .args(["-h", "+1", "Pairee: Transfer complete. Shutting down..."])
                    .spawn()?;
            }
            Ok(())
        }
        PostAction::Sleep => {
            #[cfg(target_os = "windows")]
            {
                Command::new("rundll32.exe")
                    .args(["powrprof.dll,SetSuspendState", "0", "1", "0"])
                    .spawn()?;
            }
            #[cfg(not(target_os = "windows"))]
            {
                run_systemctl("suspend")?;
            }
            Ok(())
        }
        PostAction::Hibernate => {
            #[cfg(target_os = "windows")]
            {
                Command::new("rundll32.exe")
                    .args(["powrprof.dll,SetSuspendState", "1", "1", "0"])
                    .spawn()?;
            }
            #[cfg(not(target_os = "windows"))]
            {
                run_systemctl("hibernate")?;
            }
            Ok(())
        }
        PostAction::EjectDrive(drive) => {
            #[cfg(target_os = "windows")]
            {
                let is_valid = drive.len() == 2
                    && drive
                        .chars()
                        .next()
                        .map(|c| c.is_ascii_alphabetic())
                        .unwrap_or(false)
                    && drive.chars().nth(1) == Some(':');
                let drive_letter = if is_valid { &drive } else { "D:" };
                Command::new("powershell")
                    .args(["-Command", &format!("(New-Object -ComObject Shell.Application).Namespace(17).ParseName('{}').InvokeVerb('Eject')", drive_letter)])
                    .spawn()?;
            }
            #[cfg(not(target_os = "windows"))]
            {
                // We used to default an empty `drive` to "/dev/sdb",
                // which is a dangerous "best guess" that can power off
                // an arbitrary disk on the user's system. The only sane
                // thing to do is to refuse the operation and ask the
                // caller to specify a real device path.
                if drive.trim().is_empty() {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "EjectDrive: no device path provided. \
                         Specify the /dev/sdX (or /dev/nvmeXnY) path of \
                         the drive you want to eject.",
                    ));
                }
                if !std::path::Path::new(&drive).exists() {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        format!("EjectDrive: device {} does not exist", drive),
                    ));
                }
                Command::new("udisksctl")
                    .args(["power-off", "-b", &drive])
                    .spawn()?;
            }
            Ok(())
        }
        PostAction::RunScript(path) => {
            if path.exists() {
                Command::new(&path).spawn()?;
            }
            Ok(())
        }
        PostAction::CloseApp => {
            // Cerramos de forma limpia indicando salida exitosa
            std::process::exit(0);
        }
    }
}

/// Helper that runs `systemctl <verb>` and converts the inevitable
/// "Access denied" / "not running in a graphical session" failures into
/// error messages the UI can show verbatim.
#[cfg(not(target_os = "windows"))]
fn run_systemctl(verb: &str) -> Result<(), std::io::Error> {
    use std::process::Stdio;
    let output = Command::new("systemctl")
        .arg(verb)
        .stdin(Stdio::null())
        .output()
        .map_err(|e| {
            std::io::Error::new(
                e.kind(),
                format!(
                    "systemctl is not available ({}). Power actions on this \
                     system require systemd.",
                    e
                ),
            )
        })?;
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    let code = output.status.code().unwrap_or(-1);
    Err(std::io::Error::new(
        std::io::ErrorKind::PermissionDenied,
        format!(
            "`systemctl {}` failed (exit {}): {}. \
             This usually means the user has no active logind session, \
             polkit refused the request, or systemd-logind is not running.",
            verb,
            code,
            stderr.trim()
        ),
    ))
}
