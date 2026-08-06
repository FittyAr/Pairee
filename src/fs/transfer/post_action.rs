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
                Command::new("systemctl").arg("suspend").spawn()?;
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
                Command::new("systemctl").arg("hibernate").spawn()?;
            }
            Ok(())
        }
        PostAction::EjectDrive(drive) => {
            #[cfg(target_os = "windows")]
            {
                // Drive letter is validated to be exactly `X:` (one ASCII
                // letter + colon). Even so, we avoid string-interpolating
                // the letter into a PowerShell `-Command` source string by
                // shipping a static `.ps1` and passing the letter as a
                // typed parameter. Defence in depth: a future validation
                // regression (or a non-Windows PowerShell dialect) cannot
                // turn the drive letter into a code-injection vector.
                let is_valid = drive.len() == 2
                    && drive
                        .chars()
                        .next()
                        .map(|c| c.is_ascii_alphabetic())
                        .unwrap_or(false)
                    && drive.chars().nth(1) == Some(':');
                let drive_letter = if is_valid { drive } else { "D:".to_string() };
                eject_windows_drive(&drive_letter)?;
            }
            #[cfg(not(target_os = "windows"))]
            {
                let dev = if drive.is_empty() { "/dev/sdb" } else { &drive };
                Command::new("udisksctl")
                    .args(["power-off", "-b", dev])
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

/// Eject a drive letter on Windows by running a small static PowerShell
/// script that takes the drive letter as a typed `-Drive` parameter.
/// The script body never sees the drive letter as source, so a
/// regression in the caller-side validation cannot turn it into a code
/// injection vector.
#[cfg(target_os = "windows")]
fn eject_windows_drive(drive_letter: &str) -> Result<(), std::io::Error> {
    use std::io::Write;

    let script_dir = std::env::temp_dir().join("pairee-eject");
    std::fs::create_dir_all(&script_dir)?;
    let script_path = script_dir.join("EjectDrive.ps1");
    let script_body = "param(\r\n    [Parameter(Mandatory = $true)]\r\n    [string] $Drive\r\n)\r\n$ErrorActionPreference = 'Stop'\r\n(New-Object -ComObject Shell.Application).Namespace(17).ParseName($Drive).InvokeVerb('Eject')\r\n";
    if !script_path.exists() {
        let mut f = std::fs::File::create(&script_path)?;
        f.write_all(script_body.as_bytes())?;
    }

    Command::new("powershell")
        .arg("-NoProfile")
        .arg("-ExecutionPolicy")
        .arg("Bypass")
        .arg("-File")
        .arg(&script_path)
        .arg("-Drive")
        .arg(drive_letter)
        .spawn()?;
    Ok(())
}
