//! Small path helpers shared by the UI and the actions.

use std::path::Path;

/// The last component of `path` as a `String` (empty when the path has no
/// file name, e.g. a root).
pub fn file_name_lossy(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// `path` as the user reads it. On Windows a path that starts at `/`
/// without a drive or share is never a local one: it is a server's (SFTP,
/// FTP, WebDAV, S3), and the folders joined to it there were separated
/// with `\`, so it is shown with `/` throughout (`/home/deck/umbral`, not
/// `/home\deck\umbral`). Every other path is shown as it is.
pub fn display_path(path: &Path) -> String {
    let text = path.to_string_lossy();
    if cfg!(windows) && is_server_path(&text) {
        return text.replace('\\', "/");
    }
    text.into_owned()
}

/// Starts at a root `/` with no drive (`C:`) or share (`\\server`) before.
fn is_server_path(text: &str) -> bool {
    text.starts_with('/') && !text.starts_with("//")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn server_paths_read_with_slashes() {
        assert!(is_server_path(r"/home\deck\umbral"));
        assert!(!is_server_path(r"C:\Users\me"));
        assert!(!is_server_path(r"\\server\share"));
        assert!(!is_server_path("//server/share"));
        if cfg!(windows) {
            assert_eq!(
                display_path(Path::new(r"/home\deck\umbral")),
                "/home/deck/umbral"
            );
            assert_eq!(display_path(Path::new(r"C:\Users\me")), r"C:\Users\me");
        } else {
            assert_eq!(display_path(Path::new(r"/srv/a\b")), r"/srv/a\b");
        }
    }

    #[test]
    fn names_and_roots() {
        assert_eq!(file_name_lossy(&Path::new("a").join("b.txt")), "b.txt");
        assert_eq!(file_name_lossy(Path::new("")), "");
    }
}
