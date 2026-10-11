//! What the mouse can hit: every interactive thing a frame paints records
//! its area here, and the mouse router looks the pointer up in the last
//! frame's map.
//!
//! Widgets call [`register`] while they paint; [`draw_ui`](super::draw_ui)
//! wraps the frame in a [`Recording`] that collects those calls and stores
//! them in `AppState::hits` when the frame ends. Recording goes through a
//! thread-local buffer so the hundreds of render functions (popups, kit
//! helpers, bars) do not need a hit-map parameter; outside a recording
//! [`register`] does nothing.
//!
//! Painting order is stacking order: what is registered later sits on top.
//! [`begin_layer`] marks where the popups start, so the router can tell a
//! click inside a dialog from one on the panels behind it.

use crate::app::state::tabs::TabId;
use crate::app::state::{ActivePanel, SortField};
use crate::keybindings::Action;
use crate::ui::scrollbar::types::ScrollbarHitTarget;
use crossterm::event::KeyModifiers;
use ratatui::layout::{Position, Rect};
use std::cell::RefCell;

/// Rows scrolled per mouse-wheel notch, everywhere.
pub const WHEEL_LINES: usize = 3;

/// Which part of the screen stack a hit belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum Layer {
    /// Panels, screens and the bars around them.
    #[default]
    Base,
    /// Dialogs and overlays painted on top of the base.
    Popup,
}

/// Something the mouse can act on.
#[derive(Debug, Clone, PartialEq)]
pub enum HitTarget {
    /// The whole area of a panel (frame, empty rows, footer).
    PanelBody(ActivePanel),
    /// The folder title on a panel's top border.
    PanelTitle(ActivePanel),
    /// A column title; `None` for columns that cannot sort.
    PanelHeader {
        side: ActivePanel,
        sort: Option<SortField>,
    },
    /// The row showing entry `index` of a panel.
    PanelRow { side: ActivePanel, index: usize },
    /// The border between the two panels: dragging it moves the split.
    /// `width` is the number of columns the two panels share.
    PanelSplitter { width: u16 },
    /// A cell of a panel's tab bar.
    Tab(TabId),
    /// A key of the function-key bar: `F<number>` with the modifiers whose
    /// row the bar showed.
    FKey { number: u8, modifiers: KeyModifiers },
    /// A title of the top menu bar.
    MenuTitle(usize),
    /// The top row while the menu bar is hidden: pointing there shows it.
    MenuReveal,
    /// Row `row` of drop-down (or submenu) `menu`.
    MenuItem { menu: usize, row: usize },
    /// Runs an action when clicked (update badge, transfer bar, CLI).
    Action(Action),
    /// A scrollbar track and the view its wheel scrolls.
    Scrollbar(ScrollbarHitTarget),
    /// The whole area of a dialog: a click there is "inside" even where
    /// nothing reacts to it.
    PopupBody,
    /// A dialog's top border: dragging it moves the dialog.
    PopupTitle,
    /// A dialog's bottom-right corner: dragging it resizes the dialog.
    PopupResize,
}

/// One recorded area.
#[derive(Debug, Clone, PartialEq)]
pub struct Hit {
    pub area: Rect,
    pub target: HitTarget,
    pub layer: Layer,
}

/// The hits of the last painted frame.
#[derive(Debug, Default)]
pub struct HitMap {
    hits: RefCell<Vec<Hit>>,
}

impl HitMap {
    /// The topmost hit under `(x, y)` that `accept` takes, scrollbars aside
    /// (they are matched by [`Self::scrollbars`]).
    pub fn find(&self, x: u16, y: u16, accept: impl Fn(&Hit) -> bool) -> Option<Hit> {
        let point = Position::new(x, y);
        self.hits
            .borrow()
            .iter()
            .rev()
            .filter(|hit| !matches!(hit.target, HitTarget::Scrollbar(_)))
            .find(|hit| hit.area.contains(point) && accept(hit))
            .cloned()
    }

    /// The topmost hit under `(x, y)`.
    pub fn at(&self, x: u16, y: u16) -> Option<Hit> {
        self.find(x, y, |_| true)
    }

    /// `true` when `(x, y)` is inside something painted on `layer` or above.
    pub fn covers(&self, x: u16, y: u16, layer: Layer) -> bool {
        self.find(x, y, |hit| hit.layer >= layer).is_some()
    }

    /// Scrollbars painted last frame, bottom to top.
    pub fn scrollbars(&self) -> Vec<ScrollbarHitTarget> {
        self.hits
            .borrow()
            .iter()
            .filter_map(|hit| match &hit.target {
                HitTarget::Scrollbar(target) => Some(target.clone()),
                _ => None,
            })
            .collect()
    }

    /// The first recorded hit whose target `pick` maps to a value.
    pub fn first<T>(&self, pick: impl Fn(&HitTarget) -> Option<T>) -> Option<(Rect, T)> {
        self.hits
            .borrow()
            .iter()
            .find_map(|hit| pick(&hit.target).map(|value| (hit.area, value)))
    }

    fn replace(&self, hits: Vec<Hit>) {
        *self.hits.borrow_mut() = hits;
    }
}

#[derive(Default)]
struct Buffer {
    hits: Vec<Hit>,
    layer: Layer,
}

thread_local! {
    static RECORDING: RefCell<Option<Buffer>> = const { RefCell::new(None) };
}

/// Records `target` at `area` in the frame being painted.
pub fn register(area: Rect, target: HitTarget) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    RECORDING.with_borrow_mut(|recording| {
        if let Some(buffer) = recording {
            let layer = buffer.layer;
            buffer.hits.push(Hit {
                area,
                target,
                layer,
            });
        }
    });
}

/// Everything registered from now on belongs to `layer`.
pub fn begin_layer(layer: Layer) {
    RECORDING.with_borrow_mut(|recording| {
        if let Some(buffer) = recording {
            buffer.layer = layer;
        }
    });
}

/// Collects the hits of one frame into `map` when dropped.
#[must_use = "the hits are stored when the recording is dropped"]
pub struct Recording<'a> {
    map: &'a HitMap,
}

impl<'a> Recording<'a> {
    pub fn start(map: &'a HitMap) -> Self {
        RECORDING.with_borrow_mut(|recording| *recording = Some(Buffer::default()));
        Self { map }
    }
}

impl Drop for Recording<'_> {
    fn drop(&mut self) {
        let hits = RECORDING
            .with_borrow_mut(Option::take)
            .map(|buffer| buffer.hits)
            .unwrap_or_default();
        self.map.replace(hits);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(paint: impl FnOnce()) -> HitMap {
        let map = HitMap::default();
        {
            let _recording = Recording::start(&map);
            paint();
        }
        map
    }

    #[test]
    fn later_hits_sit_on_top() {
        let map = record(|| {
            register(Rect::new(0, 0, 10, 10), HitTarget::MenuTitle(1));
            register(Rect::new(2, 2, 3, 3), HitTarget::MenuTitle(2));
        });
        assert_eq!(map.at(3, 3).unwrap().target, HitTarget::MenuTitle(2));
        assert_eq!(map.at(0, 0).unwrap().target, HitTarget::MenuTitle(1));
        assert!(map.at(20, 20).is_none());
    }

    #[test]
    fn layers_tell_dialogs_from_the_panels_behind() {
        let map = record(|| {
            register(Rect::new(0, 0, 80, 24), HitTarget::MenuTitle(1));
            begin_layer(Layer::Popup);
            register(Rect::new(10, 5, 20, 10), HitTarget::PopupBody);
        });
        assert!(map.covers(12, 6, Layer::Popup));
        assert!(!map.covers(1, 1, Layer::Popup));
        assert_eq!(map.at(12, 6).unwrap().layer, Layer::Popup);
    }

    #[test]
    fn scrollbars_are_kept_apart_from_point_lookups() {
        use crate::ui::scrollbar::ScrollTargetId;
        let bar = ScrollbarHitTarget {
            area: Rect::new(9, 0, 1, 10),
            wheel_area: Rect::new(0, 0, 10, 10),
            content_len: 50,
            viewport_len: 10,
            offset: 0,
            id: ScrollTargetId::Viewer,
        };
        let map = record(|| register(bar.wheel_area, HitTarget::Scrollbar(bar.clone())));
        assert!(map.at(9, 1).is_none());
        assert_eq!(map.scrollbars(), vec![bar]);
    }

    #[test]
    fn nothing_is_recorded_outside_a_frame() {
        register(Rect::new(0, 0, 1, 1), HitTarget::PopupBody);
        let map = record(|| {});
        assert!(map.at(0, 0).is_none());
    }

    #[test]
    fn empty_areas_are_ignored() {
        let map = record(|| register(Rect::new(0, 0, 0, 5), HitTarget::PopupBody));
        assert!(map.at(0, 0).is_none());
    }
}
