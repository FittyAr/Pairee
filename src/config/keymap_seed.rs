//! Seeds the user's `keymaps/` folder with the shipped presets without ever
//! overwriting a file the user may have edited.
//!
//! A preset file that differs from the shipped one is left alone; the new
//! shipped version is written next to it as `<name>.toml.new` (see
//! [`embedded::pending_update_path`]) and the loader warns about it. Each
//! shipped version is offered once: `.<name>.toml.seeded` keeps the hash of
//! the last version written or offered, so deleting the `.new` file after
//! merging it sticks until the next release changes the preset.

use crate::keybindings::embedded;
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

/// Writes missing presets and offers updated ones as `<name>.toml.new`.
pub fn seed_preset_keymaps(keymaps_dir: &Path) -> Result<()> {
    fs::create_dir_all(keymaps_dir).context("Failed to create keymaps directory")?;
    for (name, shipped) in embedded::PRESETS {
        if let Err(e) = seed_preset(keymaps_dir, name, shipped) {
            log::warn!("Failed to seed preset keymap '{name}': {e:#}");
        }
    }
    Ok(())
}

fn seed_preset(dir: &Path, name: &str, shipped: &str) -> Result<()> {
    let path = dir.join(format!("{name}.toml"));
    let pending = embedded::pending_update_path(dir, name);
    let stamp = stamp_path(dir, name);
    let shipped_digest = digest(shipped);
    match fs::read_to_string(&path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            fs::write(&path, shipped).with_context(|| format!("writing {}", path.display()))?;
        }
        Err(e) => return Err(e).with_context(|| format!("reading {}", path.display())),
        Ok(current) if digest(&current) == shipped_digest => {
            // Up to date: drop an offer the user no longer needs.
            let _ = fs::remove_file(&pending);
        }
        Ok(_) if fs::read_to_string(&stamp).is_ok_and(|s| s.trim() == shipped_digest) => {}
        Ok(_) => {
            fs::write(&pending, shipped)
                .with_context(|| format!("writing {}", pending.display()))?;
        }
    }
    fs::write(&stamp, &shipped_digest).with_context(|| format!("writing {}", stamp.display()))
}

/// Hidden file holding the hash of the last shipped version handled.
fn stamp_path(dir: &Path, name: &str) -> PathBuf {
    dir.join(format!(".{name}.toml.seeded"))
}

/// Hash of `text` ignoring line endings, so a checkout with CRLF and a copy
/// with LF count as the same version.
fn digest(text: &str) -> String {
    blake3::hash(text.replace("\r\n", "\n").as_bytes())
        .to_hex()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn norton() -> &'static str {
        embedded::preset_toml("norton").unwrap()
    }

    #[test]
    fn missing_presets_are_written() {
        let dir = tempfile::tempdir().unwrap();
        seed_preset_keymaps(dir.path()).unwrap();
        for (name, shipped) in embedded::PRESETS {
            let written = fs::read_to_string(dir.path().join(format!("{name}.toml"))).unwrap();
            assert_eq!(written, *shipped);
            assert!(!embedded::pending_update_path(dir.path(), name).exists());
        }
    }

    #[test]
    fn edited_preset_is_kept_and_the_update_offered_once() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("norton.toml");
        let pending = embedded::pending_update_path(dir.path(), "norton");
        let edited = "[bindings]\nquit = \"F10\"\n";
        fs::write(&path, edited).unwrap();

        seed_preset_keymaps(dir.path()).unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), edited);
        assert_eq!(fs::read_to_string(&pending).unwrap(), norton());

        // The user merged and deleted the offer: it is not written again.
        fs::remove_file(&pending).unwrap();
        seed_preset_keymaps(dir.path()).unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), edited);
        assert!(!pending.exists());
    }

    #[test]
    fn up_to_date_preset_drops_a_stale_offer() {
        let dir = tempfile::tempdir().unwrap();
        let pending = embedded::pending_update_path(dir.path(), "norton");
        let crlf = norton().replace("\r\n", "\n").replace('\n', "\r\n");
        fs::write(dir.path().join("norton.toml"), crlf).unwrap();
        fs::write(&pending, norton()).unwrap();
        seed_preset_keymaps(dir.path()).unwrap();
        assert!(!pending.exists());
    }
}
