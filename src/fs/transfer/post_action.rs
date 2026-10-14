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
/// Runs `action` once the queue is done. An [`PostAction::EjectDrive`]
/// without a drive ejects the one `destination` (where the last job wrote)
/// lies on; when that cannot be told nothing is ejected.
pub fn execute_post_action(
    action: PostAction,
    destination: Option<&std::path::Path>,
) -> Result<(), std::io::Error> {
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
            // Never a guessed default (it used to be `D:` or `/dev/sdb`,
            // whatever the transfer wrote to).
            let drive = Some(drive)
                .filter(|d| !d.is_empty())
                .or_else(|| destination.and_then(drive_of))
                .filter(|d| is_ejectable_name(d))
                .ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "no drive to eject for this transfer",
                    )
                })?;
            #[cfg(target_os = "windows")]
            eject_windows_drive(&drive)?;
            #[cfg(not(target_os = "windows"))]
            Command::new("udisksctl")
                .args(["power-off", "-b", &drive])
                .spawn()?;
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
/// The drive `path` lies on: its letter (`X:`) on Windows, the block
/// device of its mount elsewhere.
fn drive_of(path: &std::path::Path) -> Option<String> {
    #[cfg(target_os = "windows")]
    {
        match path.components().next()? {
            std::path::Component::Prefix(prefix) => match prefix.kind() {
                std::path::Prefix::Disk(letter) | std::path::Prefix::VerbatimDisk(letter) => {
                    Some(format!("{}:", char::from(letter)))
                }
                _ => None,
            },
            _ => None,
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        super::network::mount_of(path).map(|(device, _)| device)
    }
}

/// A drive name that is safe to hand to the eject command: exactly `X:` on
/// Windows (it reaches PowerShell, see [`eject_windows_drive`]), a device
/// below `/dev/` elsewhere.
fn is_ejectable_name(drive: &str) -> bool {
    #[cfg(target_os = "windows")]
    {
        let bytes = drive.as_bytes();
        bytes.len() == 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':'
    }
    #[cfg(not(target_os = "windows"))]
    {
        drive.starts_with("/dev/") && !drive.contains("..")
    }
}

/// The drive letter is checked by [`is_ejectable_name`]; it also reaches
/// PowerShell as a quoted literal, never as code.
#[cfg(target_os = "windows")]
fn eject_windows_drive(drive_letter: &str) -> Result<(), std::io::Error> {
    crate::fs::privileges::powershell(
        "$ErrorActionPreference = 'Stop'; (New-Object -ComObject Shell.Application).Namespace(17).ParseName($v0).InvokeVerb('Eject')",
        &[std::ffi::OsStr::new(drive_letter)],
    )
    .map_err(std::io::Error::other)?
    .spawn()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Without a drive and a destination to tell it from, nothing is
    /// ejected (it used to power off `/dev/sdb` or eject `D:`).
    #[test]
    fn eject_without_a_known_drive_does_nothing() {
        let result = execute_post_action(PostAction::EjectDrive(String::new()), None);
        assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::InvalidInput);
    }

    #[test]
    fn only_plain_drive_names_are_ejected() {
        #[cfg(target_os = "windows")]
        {
            assert!(is_ejectable_name("E:"));
            assert!(!is_ejectable_name("E:; rm"));
            assert_eq!(
                drive_of(std::path::Path::new(r"F:\backup\a.txt")).as_deref(),
                Some("F:")
            );
        }
        #[cfg(not(target_os = "windows"))]
        {
            assert!(is_ejectable_name("/dev/sdc1"));
            assert!(!is_ejectable_name("tmpfs"));
        }
    }
}
