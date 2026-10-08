//! Name keys used to match entries of the two sides.

/// Whether the platform's usual filesystems ignore case in file names
/// (NTFS/FAT on Windows, APFS/HFS+ on macOS).
pub const fn platform_case_insensitive() -> bool {
    cfg!(any(windows, target_os = "macos"))
}

/// Key under which `name` is matched against the other side.
pub fn name_key(name: &str, case_insensitive: bool) -> String {
    if case_insensitive {
        name.to_lowercase()
    } else {
        name.to_owned()
    }
}
