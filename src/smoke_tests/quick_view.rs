//! Quick view beside the panels: the panels keep their keys, and dialogs
//! opened while it shows are not closed by it.

use crate::app::state::PopupType;
use crate::app::state::popup::MkDirPrompt;
use crate::test_harness::Harness;

fn quick_view_path(h: &Harness) -> Option<std::path::PathBuf> {
    match h.state.dialogs.top() {
        Some(PopupType::QuickViewPanel(qv)) => Some(qv.path.clone()),
        _ => None,
    }
}

#[test]
fn panel_keys_and_dialogs_work_while_the_quick_view_shows() {
    let mut h = Harness::new();
    h.write("work/left/a.txt", "a");
    h.write("work/left/b.txt", "b");
    h.reread().focus("a.txt").keys("@quick_view");
    assert_eq!(quick_view_path(&h), Some(h.left().join("a.txt")));

    h.keys("Down");
    assert_eq!(h.cursor_name(), "b.txt", "the panel moved");
    assert_eq!(quick_view_path(&h), Some(h.left().join("b.txt")));

    h.keys("@mkdir");
    assert!(
        matches!(
            h.state.dialogs.top(),
            Some(PopupType::MkDirPrompt(MkDirPrompt { .. }))
        ),
        "{:?}",
        h.state.dialogs.top()
    );
    h.keys("Esc");
    assert_eq!(quick_view_path(&h), Some(h.left().join("b.txt")), "back");
    h.keys("Esc");
    assert!(h.state.dialogs.top().is_none());
    assert!(!h.state.panels.quick_view_active);
}

fn quick_view_scroll(h: &Harness) -> usize {
    match h.state.dialogs.top() {
        Some(PopupType::QuickViewPanel(qv)) => qv.scroll,
        other => panic!("{other:?}"),
    }
}

/// Opens the quick view on the cursor file and waits for its text.
fn open_preview(h: &mut Harness) {
    h.keys("@quick_view");
    h.wait_until("the preview", |h| {
        matches!(h.state.dialogs.top(), Some(PopupType::QuickViewPanel(qv)) if qv.content.len() > 1)
    });
}

/// The furthest PgDn goes, as painted: the scrollbar's content and viewport.
fn painted_last_offset(h: &mut Harness) -> usize {
    use crate::ui::hit::HitTarget;
    use crate::ui::scrollbar::ScrollTargetId;
    h.render();
    h.state
        .hits
        .first(|t| match t {
            HitTarget::Scrollbar(bar) if bar.id == ScrollTargetId::QuickView => {
                Some(bar.content_len - bar.viewport_len)
            }
            _ => None,
        })
        .map(|(_, last)| last)
        .unwrap_or_else(|| {
            panic!(
                "no quick view scrollbar
{}",
                h.screen_text()
            )
        })
}

/// PgDn stops at the last screenful of the panel as drawn, whatever the
/// terminal height (it used to assume 20 rows).
#[test]
fn quick_view_scroll_stops_at_the_rendered_viewport() {
    let text: String = (0..200).map(|i| format!("line {i}\n")).collect();
    for height in [20, 30, 60] {
        let mut h = Harness::builder().size(100, height).build();
        h.write("work/left/long.txt", &text);
        h.reread().focus("long.txt");
        open_preview(&mut h);
        let last = painted_last_offset(&mut h);
        h.keys("PageDown*30");
        assert_eq!(quick_view_scroll(&h), last, "height {height}");
        h.render().assert_screen("line 199");
    }
}

/// A plugin's list scrolls by its own rows, not the built-in preview's.
#[test]
fn plugin_widget_scroll_uses_the_widget_length() {
    use crate::app::state::popup::plugin_widget::{WList, WText};
    use crate::app::state::popup::{PluginPreview, Renderable};
    let text: String = (0..500).map(|i| format!("line {i}\n")).collect();
    let mut h = Harness::new();
    h.write("work/left/long.txt", &text);
    h.reread().focus("long.txt");
    open_preview(&mut h);
    let Some(PopupType::QuickViewPanel(qv)) = h.state.dialogs.top_mut() else {
        panic!("quick view");
    };
    qv.plugin = Some(PluginPreview {
        widget: Renderable::List(WList {
            items: (0..40).map(|i| WText::raw(&format!("item {i}"))).collect(),
            block: None,
            style: Default::default(),
        }),
        plugin: "demo".into(),
        skip: 0,
        seekable: false,
    });
    let last = painted_last_offset(&mut h);
    assert!(last < 40);
    h.keys("PageDown*10");
    assert_eq!(quick_view_scroll(&h), last);
    h.render().assert_screen("item 39");
}
