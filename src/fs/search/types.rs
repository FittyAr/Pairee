use std::path::PathBuf;

/// Search query target types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchTarget {
    Any,
    File,
    Directory,
}

/// Search query parameters.
#[derive(Debug, Clone)]
pub struct SearchQuery {
    /// Glob name pattern (e.g. "*.rs"). Empty string means match all.
    pub name_glob: String,
    /// Optional text to search inside file content. None = skip content search.
    pub content: Option<String>,
    /// Root directory to start the recursive search.
    pub root: PathBuf,
    /// Whether glob name matching and content search are case-sensitive.
    pub case_sensitive: bool,
    /// Target type of entries to find.
    pub target: SearchTarget,
}
