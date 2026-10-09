//! Per-panel back / forward history of visited folders (browser style:
//! Alt+←/→, Vim's Ctrl+o / Ctrl+i, yazi's H / L).

use std::path::{Path, PathBuf};

/// Folders kept on each side.
const LIMIT: usize = 64;

#[derive(Debug, Default)]
pub struct NavHistory {
    back: Vec<PathBuf>,
    forward: Vec<PathBuf>,
    /// The folder a back / forward step is heading to, so arriving there
    /// does not count as a new visit.
    travelling_to: Option<PathBuf>,
}

/// Which way to travel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Travel {
    Back,
    Forward,
}

impl NavHistory {
    /// The panel moved from `from` to `to`.
    pub fn visited(&mut self, from: PathBuf, to: &Path) {
        if self.travelling_to.take().as_deref() == Some(to) {
            return;
        }
        if self.back.last() != Some(&from) {
            self.back.push(from);
            if self.back.len() > LIMIT {
                self.back.remove(0);
            }
        }
        self.forward.clear();
    }

    /// The folder to open for a step `way` from `current`, if any.
    pub fn travel(&mut self, way: Travel, current: PathBuf) -> Option<PathBuf> {
        let (from, to) = match way {
            Travel::Back => (&mut self.back, &mut self.forward),
            Travel::Forward => (&mut self.forward, &mut self.back),
        };
        let target = from.pop()?;
        to.push(current);
        self.travelling_to = Some(target.clone());
        Some(target)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> PathBuf {
        PathBuf::from(s)
    }

    #[test]
    fn back_and_forward_retrace_visits() {
        let mut h = NavHistory::default();
        h.visited(p("/a"), &p("/b"));
        h.visited(p("/b"), &p("/c"));
        assert_eq!(h.travel(Travel::Back, p("/c")), Some(p("/b")));
        h.visited(p("/c"), &p("/b"));
        assert_eq!(h.travel(Travel::Back, p("/b")), Some(p("/a")));
        h.visited(p("/b"), &p("/a"));
        assert_eq!(h.travel(Travel::Back, p("/a")), None);
        assert_eq!(h.travel(Travel::Forward, p("/a")), Some(p("/b")));
        h.visited(p("/a"), &p("/b"));
        assert_eq!(h.travel(Travel::Forward, p("/b")), Some(p("/c")));
    }

    #[test]
    fn a_new_visit_drops_the_forward_list() {
        let mut h = NavHistory::default();
        h.visited(p("/a"), &p("/b"));
        h.travel(Travel::Back, p("/b"));
        h.visited(p("/b"), &p("/a"));
        h.visited(p("/a"), &p("/z"));
        assert_eq!(h.travel(Travel::Forward, p("/z")), None);
        assert_eq!(h.travel(Travel::Back, p("/z")), Some(p("/a")));
    }
}
