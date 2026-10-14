//! The folder a shell wrapper should `cd` to when Pairee exits
//! (`--cwd-file`, `--print-cwd`).

use crate::app::state::PanelState;
use crate::fs::vfs::PanelSource;
use std::path::{Path, PathBuf};

/// The focused panel's local folder: the folder itself, or the folder
/// holding the archive it browses. SFTP panels have no local folder.
pub fn exit_dir(panel: &PanelState) -> Option<PathBuf> {
    match &panel.source {
        PanelSource::Local => Some(panel.current_path.clone()),
        PanelSource::Archive(_) => panel
            .current_path
            .ancestors()
            .find(|p| p.is_file())
            .and_then(Path::parent)
            .map(Path::to_path_buf),
        PanelSource::Remote(_) => None,
    }
}

/// Writes `dir` to `file` (no trailing newline, as shell wrappers expect);
/// without a local folder the file is left empty so the shell stays put.
/// The path is written byte for byte, so a non-UTF-8 folder name on Unix
/// reaches the shell intact.
pub fn write_cwd_file(file: &Path, dir: Option<&Path>) -> std::io::Result<()> {
    let bytes = dir.map_or(&[][..], |d| d.as_os_str().as_encoded_bytes());
    std::fs::write(file, bytes)
}

/// Prints `dir` and a newline to stdout, byte for byte (see
/// [`write_cwd_file`]).
pub fn print_dir(dir: &Path) -> std::io::Result<()> {
    use std::io::Write;
    let mut out = std::io::stdout().lock();
    out.write_all(dir.as_os_str().as_encoded_bytes())?;
    out.write_all(b"\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_panel_exits_in_its_folder_and_file_is_written() {
        let dir = tempfile::tempdir().unwrap();
        let panel = PanelState::new(dir.path().to_path_buf());
        let exit = exit_dir(&panel).unwrap();
        assert_eq!(exit, dir.path());

        let file = dir.path().join("cwd");
        write_cwd_file(&file, Some(&exit)).unwrap();
        assert_eq!(
            std::fs::read_to_string(&file).unwrap(),
            dir.path().to_string_lossy()
        );
        write_cwd_file(&file, None).unwrap();
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "");
    }

    #[cfg(unix)]
    #[test]
    fn a_non_utf8_folder_is_written_byte_for_byte() {
        use std::os::unix::ffi::OsStrExt;
        let dir = tempfile::tempdir().unwrap();
        let odd = Path::new(std::ffi::OsStr::from_bytes(b"/tmp/caf\xe9"));
        let file = dir.path().join("cwd");
        write_cwd_file(&file, Some(odd)).unwrap();
        assert_eq!(std::fs::read(&file).unwrap(), b"/tmp/caf\xe9");
    }
}
