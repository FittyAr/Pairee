//! Find file runs as a background job: matches fill the results list and
//! Esc cancels the search.

use crate::app::state::PopupType;
use crate::test_harness::Harness;

fn results(h: &Harness) -> Option<(usize, bool)> {
    match h.state.dialogs.top() {
        Some(PopupType::SearchResults {
            results, searching, ..
        }) => Some((results.len(), *searching)),
        _ => None,
    }
}

#[test]
fn find_file_lists_matches_then_esc_cancels() {
    let mut h = Harness::new();
    h.write("work/left/needle_one.txt", "x");
    h.write("work/left/sub/needle_two.txt", "x");
    h.write("work/left/other.txt", "x");
    h.reread().keys("@find_file");
    h.text("needle").keys("Enter");
    h.wait_until("search finished", |h| results(h) == Some((2, false)));
    assert!(!h.state.search.is_running());

    h.keys("Esc");
    assert!(results(&h).is_none());
}
