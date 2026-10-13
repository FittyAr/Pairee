//! Values the renderer derives from the configuration (the parsed theme,
//! the compiled highlight rules). Each is rebuilt only when its source
//! changes, whatever code path changed it, and shared between frames.

use crate::config::theme::Theme;
use crate::ui::highlight::{HighlightRule, Highlighter};
use crate::ui::resolved_theme::ResolvedTheme;
use std::borrow::Borrow;
use std::sync::{Arc, Mutex};

/// `D` built from a source `S`, rebuilt when the source differs from the
/// one it was built from.
pub struct Derived<S: ToOwned + ?Sized, D> {
    slot: Mutex<Option<(S::Owned, Arc<D>)>>,
}

impl<S: ToOwned + ?Sized, D> Default for Derived<S, D> {
    fn default() -> Self {
        Self {
            slot: Mutex::new(None),
        }
    }
}

impl<S: ToOwned + PartialEq + ?Sized, D> Derived<S, D> {
    /// The value for `source`, built with `build` only when it changed.
    pub fn get(&self, source: &S, build: impl FnOnce(&S) -> D) -> Arc<D> {
        let mut slot = self.slot.lock().unwrap_or_else(|e| e.into_inner());
        match slot.as_ref() {
            Some((built_from, value)) if built_from.borrow() == source => Arc::clone(value),
            _ => {
                let value = Arc::new(build(source));
                *slot = Some((source.to_owned(), Arc::clone(&value)));
                value
            }
        }
    }
}

/// The derived render values of an [`crate::app::context::AppContext`].
#[derive(Default)]
pub struct RenderCache {
    theme: Derived<Theme, ResolvedTheme>,
    highlighter: Derived<[HighlightRule], Highlighter>,
}

impl RenderCache {
    pub fn theme(&self, theme: &Theme) -> Arc<ResolvedTheme> {
        self.theme.get(theme, ResolvedTheme::new)
    }

    pub fn highlighter(&self, rules: &[HighlightRule]) -> Arc<Highlighter> {
        self.highlighter.get(rules, Highlighter::new)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    #[test]
    fn rebuilds_only_when_the_source_changes() {
        let derived: Derived<str, usize> = Derived::default();
        let builds = Cell::new(0);
        let build = |s: &str| {
            builds.set(builds.get() + 1);
            s.len()
        };
        assert_eq!(*derived.get("ab", build), 2);
        assert_eq!(*derived.get("ab", build), 2);
        assert_eq!(builds.get(), 1);
        assert_eq!(*derived.get("abc", build), 3);
        assert_eq!(builds.get(), 2);
    }

    #[test]
    fn theme_follows_config_changes() {
        let cache = RenderCache::default();
        let slate = cache.theme(&Theme::default());
        let blue = cache.theme(&Theme::classic_blue());
        assert_ne!(*slate, *blue);
        assert!(Arc::ptr_eq(&blue, &cache.theme(&Theme::classic_blue())));
    }
}
