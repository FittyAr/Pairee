//! The one "contains, ignoring case" test behind every type-to-filter list
//! (quick filter of a panel, task list, plugin registry, shortcuts list).

/// A typed filter, lower-cased once; an empty filter matches everything.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TextFilter {
    needle: String,
}

impl TextFilter {
    pub fn new(query: &str) -> Self {
        Self {
            needle: query.trim().to_lowercase(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.needle.is_empty()
    }

    /// `text` contains the filter, ignoring case.
    pub fn matches(&self, text: &str) -> bool {
        self.needle.is_empty() || text.to_lowercase().contains(&self.needle)
    }

    /// Any of `texts` contains the filter.
    pub fn matches_any<'a>(&self, texts: impl IntoIterator<Item = &'a str>) -> bool {
        self.needle.is_empty() || texts.into_iter().any(|t| self.matches(t))
    }

    /// Moves the items matching the filter (by `key`) before the others,
    /// keeping their order within each group.
    pub fn matching_first<T>(&self, items: Vec<T>, key: impl Fn(&T) -> &str) -> Vec<T> {
        if self.is_empty() {
            return items;
        }
        let (mut first, rest): (Vec<T>, Vec<T>) =
            items.into_iter().partition(|item| self.matches(key(item)));
        first.extend(rest);
        first
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_ignoring_case_and_empty_matches_all() {
        let f = TextFilter::new(" ReAd ");
        assert!(f.matches("README.md"));
        assert!(!f.matches("main.rs"));
        assert!(f.matches_any(["x", "already"]));
        assert!(TextFilter::new("").matches("anything"));
        assert!(TextFilter::new("ñ").matches("AÑO"));
    }

    #[test]
    fn matching_first_keeps_order() {
        let f = TextFilter::new("a");
        let out = f.matching_first(vec!["xb", "a1", "c", "ba"], |s| s);
        assert_eq!(out, vec!["a1", "ba", "xb", "c"]);
    }
}
