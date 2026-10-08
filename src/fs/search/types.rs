use std::path::PathBuf;

/// Search query target types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchTarget {
    Any,
    File,
    Directory,
}

impl SearchTarget {
    const ORDER: [Self; 3] = [Self::Any, Self::File, Self::Directory];

    /// Next (or previous) target, wrapping.
    pub fn cycle(self, forward: bool) -> Self {
        let idx = Self::ORDER.iter().position(|&t| t == self).unwrap_or(0);
        let len = Self::ORDER.len();
        Self::ORDER[if forward {
            (idx + 1) % len
        } else {
            (idx + len - 1) % len
        }]
    }

    /// Label translation key.
    pub fn label_key(self) -> &'static str {
        match self {
            Self::Any => "search_target_any",
            Self::File => "search_target_file",
            Self::Directory => "search_target_dir",
        }
    }
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
