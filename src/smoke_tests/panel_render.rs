//! Panel colors: the user's "Files highlighting" rules and theme changes
//! reach the rendered rows.

use crate::ui::highlight::HighlightRule;
use ratatui::style::Color;

use crate::test_harness::Harness;

#[test]
fn panel_uses_the_users_highlight_rules() {
    let mut h = Harness::builder()
        .settings(|s| {
            s.highlight_rules = vec![HighlightRule::new("*.xyz", "Magenta")];
        })
        .build();
    h.write("work/left/data.xyz", "x");
    h.write("work/left/main.rs", "x");
    h.reread().focus("..");
    let (col, row) = h.locate("data.xyz");
    assert_eq!(h.cell_style(col, row).fg, Some(Color::Magenta));
    // The built-in rule for sources is gone: the user replaced the list.
    let (col, row) = h.locate("main.rs");
    assert_ne!(h.cell_style(col, row).fg, Some(Color::LightGreen));

    // Edited rules apply on the next frame.
    h.ctx.config.settings.highlight_rules = vec![HighlightRule::new("*.rs", "LightRed")];
    assert_eq!(h.cell_style(col, row).fg, Some(Color::LightRed));
}

#[test]
fn theme_changes_reach_the_next_frame() {
    let mut h = Harness::new();
    h.write("work/left/a.txt", "x");
    h.reread().focus("a.txt");
    let (col, row) = h.locate("a.txt");
    assert_eq!(h.cell_style(col, row).bg, Some(Color::Blue));
    h.ctx.config.theme = crate::config::theme::Theme::classic_blue();
    assert_eq!(h.cell_style(col, row).bg, Some(Color::Rgb(0, 0xAA, 0xAA)));
}
