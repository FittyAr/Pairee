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
