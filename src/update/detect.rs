/// Detect how Pairee was installed on the current machine.
///
/// The detection is done heuristically by inspecting:
/// - Environment variables (SNAP, FLATPAK_ID, etc.)
/// - Registry keys on Windows
/// - Presence of package manager databases
/// - Location of the current executable

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum InstallMethod {
    // ── Linux ──────────────────────────────────────────────────────────────
    /// Installed via the official install.sh script or extracted tar.gz manually.
    /// The binary lives in ~/.local/bin or any path not managed by a package manager.
    #[cfg_attr(target_os = "windows", allow(dead_code))]
    TarballManual,
    /// Installed via a .deb package (apt, dpkg).
    #[cfg_attr(target_os = "windows", allow(dead_code))]
    Deb,
    /// Installed via a .rpm package (dnf, zypper, rpm).
    #[cfg_attr(target_os = "windows", allow(dead_code))]
    Rpm,
    /// Installed via the AUR (yay, paru, makepkg) on Arch / Manjaro / EndeavourOS.
    #[cfg_attr(target_os = "windows", allow(dead_code))]
    AurPacman,
    /// Installed via Nix or NixOS package manager.
    #[cfg_attr(target_os = "windows", allow(dead_code))]
    Nix,
    /// Installed as a Snap package.
    #[cfg_attr(target_os = "windows", allow(dead_code))]
    Snap,
    /// Installed as a Flatpak.
    #[cfg_attr(target_os = "windows", allow(dead_code))]
    Flatpak,

    // ── Windows ────────────────────────────────────────────────────────────
    // Nota: Las siguientes variantes solo se construyen/detectan en Windows.
    // Se permite dead_code en sistemas no-Windows para mantener la estructura del enum compartida globalmente.
    /// Installed via the official install.ps1 script or extracted zip manually.
    #[cfg_attr(not(target_os = "windows"), allow(dead_code))]
    ZipManual,
    /// Installed via the Inno Setup .exe installer.
    #[cfg_attr(not(target_os = "windows"), allow(dead_code))]
    InnoSetup,
    /// Installed via winget.
    #[cfg_attr(not(target_os = "windows"), allow(dead_code))]
    Winget,
    /// Installed via Scoop.
    #[cfg_attr(not(target_os = "windows"), allow(dead_code))]
    Scoop,
    /// Installed via Chocolatey.
    #[cfg_attr(not(target_os = "windows"), allow(dead_code))]
    Chocolatey,

    /// Could not determine the install method.
    #[allow(dead_code)]
    Unknown,
}

impl InstallMethod {
    /// Returns true if the update must be performed by an external package manager.
    /// In this case Pairee should show a command rather than downloading itself.
    pub fn is_managed(&self) -> bool {
        matches!(
            self,
            Self::AurPacman
                | Self::Nix
                | Self::Snap
                | Self::Flatpak
                | Self::Winget
                | Self::Scoop
                | Self::Chocolatey
                | Self::Deb
                | Self::Rpm
        )
    }

    /// Returns the exact shell command the user should run to upgrade Pairee,
    /// or None if the update is self-managed.
    pub fn managed_upgrade_command(&self) -> Option<String> {
        match self {
            Self::AurPacman => Some("yay -Syu pairee  # or: paru -Syu pairee".to_string()),
            Self::Nix => Some("nix-env -u pairee  # or update your flake inputs".to_string()),
            Self::Snap => Some("sudo snap refresh pairee".to_string()),
            Self::Flatpak => Some("flatpak update io.github.fittyar.Pairee".to_string()),
            Self::Winget => Some("winget upgrade FittyAr.Pairee".to_string()),
            Self::Scoop => Some("scoop update pairee".to_string()),
            Self::Chocolatey => Some("choco upgrade pairee".to_string()),
            Self::Deb => Some("sudo apt-get install --only-upgrade pairee".to_string()),
            Self::Rpm => {
                Some("sudo dnf upgrade pairee  # or: sudo zypper update pairee".to_string())
            }
            _ => None,
        }
    }

    /// Human-readable label for UI display.
    pub fn label(&self) -> &'static str {
        match self {
            Self::TarballManual => "tarball (manual)",
            Self::Deb => ".deb package",
            Self::Rpm => ".rpm package",
            Self::AurPacman => "AUR / pacman",
            Self::Nix => "Nix / NixOS",
            Self::Snap => "Snap",
            Self::Flatpak => "Flatpak",
            Self::ZipManual => "zip (manual)",
            Self::InnoSetup => "Windows installer",
            Self::Winget => "winget",
            Self::Scoop => "Scoop",
            Self::Chocolatey => "Chocolatey",
            Self::Unknown => "unknown",
        }
    }
}

static DETECTED_INSTALL_METHOD: std::sync::OnceLock<InstallMethod> = std::sync::OnceLock::new();

/// Detect the install method of the currently running Pairee binary.
pub fn detect_install_method() -> InstallMethod {
    *DETECTED_INSTALL_METHOD.get_or_init(|| {
        #[cfg(target_os = "windows")]
        {
            detect_windows()
        }
        #[cfg(not(target_os = "windows"))]
        {
            detect_linux()
        }
    })
}

// ─── Linux detection ─────────────────────────────────────────────────────────

#[cfg(not(target_os = "windows"))]
fn detect_linux() -> InstallMethod {
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
        let exe_str = exe.to_string_lossy();
        if exe_str.starts_with("/nix/store") || exe_str.contains("/nix/") {
            return InstallMethod::Nix;
        }
        // 4. AUR / pacman: query pacman database
        if is_command_available("pacman") {
            if is_pacman_owned(&exe) {
                return InstallMethod::AurPacman;
            }
        }

        // 5. dpkg: query dpkg database
        if is_command_available("dpkg") {
            if is_dpkg_owned(&exe) {
                return InstallMethod::Deb;
            }
        }

        // 6. rpm: query rpm database
        if is_command_available("rpm") {
            if is_rpm_owned(&exe) {
                return InstallMethod::Rpm;
            }
        }
    }

    // Default for Linux: assume manual tarball install
    InstallMethod::TarballManual
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

// ─── Windows detection ───────────────────────────────────────────────────────

#[cfg(target_os = "windows")]
fn detect_windows() -> InstallMethod {
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
    // Check if winget is present and if it lists pairee
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
    // Look for unins*.exe next to the running exe
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let name = entry.file_name();
                    let name_str = name.to_string_lossy().to_lowercase();
                    if name_str.starts_with("unins") && name_str.ends_with(".exe") {
                        return true;
                    }
                }
            }
        }
    }
    false
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn managed_methods_have_commands() {
        let methods = [
            InstallMethod::AurPacman,
            InstallMethod::Nix,
            InstallMethod::Snap,
            InstallMethod::Flatpak,
            InstallMethod::Winget,
            InstallMethod::Scoop,
            InstallMethod::Chocolatey,
            InstallMethod::Deb,
            InstallMethod::Rpm,
        ];
        for m in &methods {
            assert!(m.is_managed());
            assert!(
                m.managed_upgrade_command().is_some(),
                "{:?} should have a command",
                m
            );
        }
    }

    #[test]
    fn self_managed_methods_have_no_command() {
        let methods = [
            InstallMethod::TarballManual,
            InstallMethod::ZipManual,
            InstallMethod::InnoSetup,
            InstallMethod::Unknown,
        ];
        for m in &methods {
            assert!(!m.is_managed(), "{:?} should not be managed", m);
        }
    }
}
