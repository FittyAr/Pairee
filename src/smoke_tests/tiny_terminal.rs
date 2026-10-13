//! Terminals below the minimum size show a "too small" notice instead of a
//! squeezed layout; at the minimum size every dialog still draws.

use crate::config::localization::t;
use crate::test_harness::Harness;
use crate::ui::too_small::{MIN_HEIGHT, MIN_WIDTH};

fn notice_start() -> String {
    t("terminal_too_small")
        .split_whitespace()
        .next()
        .unwrap_or_default()
        .to_string()
}

#[test]
fn below_the_minimum_the_screen_asks_for_more_room() {
    let mut h = Harness::new();
    h.write("work/left/visible.txt", "x");
    h.reread();
    h.resize(MIN_WIDTH - 1, MIN_HEIGHT + 4);
    h.assert_screen(&notice_start());
    h.assert_no_screen("visible.txt");
    h.resize(100, 30);
    h.assert_screen("visible.txt");
}

#[test]
fn dialogs_draw_at_the_minimum_size() {
    let mut h = Harness::new();
    h.resize(MIN_WIDTH, MIN_HEIGHT);
    for script in ["@help", "@screens_list", "@find_file", "@menu"] {
        h.keys(script).render();
        h.keys("Esc Esc");
    }
    h.resize(1, 1);
    h.render();
}
