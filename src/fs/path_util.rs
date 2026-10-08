//! Small path helpers shared by the UI and the actions.

use std::path::Path;

/// The last component of `path` as a `String` (empty when the path has no
/// file name, e.g. a root).
pub fn file_name_lossy(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_and_roots() {
        assert_eq!(file_name_lossy(&Path::new("a").join("b.txt")), "b.txt");
        assert_eq!(file_name_lossy(Path::new("")), "");
    }
}
