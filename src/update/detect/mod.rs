pub mod os;
pub mod types;

pub use types::InstallMethod;

static DETECTED_INSTALL_METHOD: std::sync::OnceLock<InstallMethod> = std::sync::OnceLock::new();

/// Detect the install method of the currently running Pairee binary.
pub fn detect_install_method() -> InstallMethod {
    *DETECTED_INSTALL_METHOD.get_or_init(os::detect_os)
}

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
