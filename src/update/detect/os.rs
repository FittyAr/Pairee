use super::types::InstallMethod;

#[cfg(not(target_os = "windows"))]
pub fn detect_os() -> InstallMethod {
    // 1. Snap: the runtime sets $SNAP
    if std::env::var("SNAP").is_ok() || std::env::var("SNAP_NAME").is_ok() {
        return InstallMethod::Snap;
    }

    // 2. Flatpak: the runtime sets $FLATPAK_ID
    if std::env::var("FLATPAK_ID").is_ok() {
        return InstallMethod::Flatpak;
    }

    // 3. Nix: check if exe is under /nix/store or managed by nix-env
    if let Ok(exe) = std::env::current_exe() {
        if is_nix_path(&exe, std::env::var_os("HOME").as_deref()) {
            return InstallMethod::Nix;
        }
        // 4. AUR / pacman: query pacman database
        if is_command_available("pacman") && is_pacman_owned(&exe) {
            return InstallMethod::AurPacman;
        }

        // 5. dpkg: query dpkg database
        if is_command_available("dpkg") && is_dpkg_owned(&exe) {
            return InstallMethod::Deb;
        }

        // 6. rpm: query rpm database
        if is_command_available("rpm") && is_rpm_owned(&exe) {
            return InstallMethod::Rpm;
        }
    }

    // Default for Linux: assume manual tarball install
    InstallMethod::TarballManual
}

/// True for binaries managed by Nix: the store itself, or a profile symlink
/// (`~/.nix-profile`, `/nix/var/nix/profiles`, `/run/current-system`).
#[cfg(any(not(target_os = "windows"), test))]
fn is_nix_path(exe: &std::path::Path, home: Option<&std::ffi::OsStr>) -> bool {
    exe.starts_with("/nix/store")
        || exe.starts_with("/nix/var/nix/profiles")
        || exe.starts_with("/run/current-system")
        || home.is_some_and(|h| exe.starts_with(std::path::Path::new(h).join(".nix-profile")))
}

#[cfg(not(target_os = "windows"))]
fn is_command_available(cmd: &str) -> bool {
    std::process::Command::new("which")
        .arg(cmd)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[cfg(not(target_os = "windows"))]
fn is_pacman_owned(exe: &std::path::Path) -> bool {
    std::process::Command::new("pacman")
        .args(["-Qo", &exe.to_string_lossy()])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[cfg(not(target_os = "windows"))]
fn is_dpkg_owned(exe: &std::path::Path) -> bool {
    std::process::Command::new("dpkg")
        .args(["-S", &exe.to_string_lossy()])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[cfg(not(target_os = "windows"))]
fn is_rpm_owned(exe: &std::path::Path) -> bool {
    std::process::Command::new("rpm")
        .args(["-qf", &exe.to_string_lossy()])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[cfg(target_os = "windows")]
pub fn detect_os() -> InstallMethod {
    // 1. Scoop: installed under %USERPROFILE%\scoop\apps\pairee
    if let Some(profile) = dirs_home() {
        let scoop_path = profile.join("scoop").join("apps").join("pairee");
        if scoop_path.exists() {
            return InstallMethod::Scoop;
        }
    }

    // 2. Chocolatey: installed under %ChocolateyInstall%\lib\pairee
    if let Ok(choco_install) = std::env::var("ChocolateyInstall") {
        let choco_path = std::path::PathBuf::from(choco_install)
            .join("lib")
            .join("pairee");
        if choco_path.exists() {
            return InstallMethod::Chocolatey;
        }
    }

    // 3. Winget: check winget list (may be slow, only if winget is present)
    if is_winget_managed() {
        return InstallMethod::Winget;
    }

    // 4. Inno Setup: look for uninstall registry key or unins000.exe
    if is_inno_setup_install() {
        return InstallMethod::InnoSetup;
    }

    // Default: zip / PowerShell manual install
    InstallMethod::ZipManual
}

#[cfg(target_os = "windows")]
fn dirs_home() -> Option<std::path::PathBuf> {
    std::env::var("USERPROFILE")
        .ok()
        .map(std::path::PathBuf::from)
}

#[cfg(target_os = "windows")]
fn is_winget_managed() -> bool {
    let out = std::process::Command::new("winget")
        .args(["list", "--id", "FittyAr.Pairee", "--exact"])
        .output();
    match out {
        Ok(o) => {
            o.status.success() && String::from_utf8_lossy(&o.stdout).contains("FittyAr.Pairee")
        }
        Err(_) => false,
    }
}

#[cfg(target_os = "windows")]
fn is_inno_setup_install() -> bool {
    if let Ok(exe) = std::env::current_exe()
        && let Some(dir) = exe.parent()
        && let Ok(entries) = std::fs::read_dir(dir)
    {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy().to_lowercase();
            if name_str.starts_with("unins") && name_str.ends_with(".exe") {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn nix_paths_are_recognised_strictly() {
        let home = Some(std::ffi::OsStr::new("/home/u"));
        assert!(is_nix_path(
            Path::new("/nix/store/abc-pairee/bin/pairee"),
            home
        ));
        assert!(is_nix_path(
            Path::new("/home/u/.nix-profile/bin/pairee"),
            home
        ));
        assert!(!is_nix_path(Path::new("/home/u/nix/tools/pairee"), home));
        assert!(!is_nix_path(Path::new("/opt/nix/pairee"), home));
        assert!(!is_nix_path(
            Path::new("/home/u/.nix-profile/bin/pairee"),
            None
        ));
    }
}
