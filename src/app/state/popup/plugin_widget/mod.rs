//! What plugin previewers draw: the [`Renderable`] tree a `peek` (or
//! `pairee.preview_widget`) hands to the host. Plain data with no ratatui
//! types; `ui::quickview::plugin` maps it to ratatui widgets.

pub mod geometry;
mod text;

pub use geometry::{WBar, WFill, WPad, WRect};
pub use text::{WAlign, WLine, WSpan, WStyle, WText, WTextBody, modifier};

use std::path::PathBuf;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WWrap {
    No,
    Yes,
    /// Wrap and trim leading spaces of wrapped lines.
    Trim,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WBorderType {
    Plain,
    Rounded,
    Double,
    Thick,
}

/// Which sides of a border are drawn (`ui.Edge` bits).
pub mod edge {
    pub const TOP: u8 = 1;
    pub const RIGHT: u8 = 1 << 1;
    pub const BOTTOM: u8 = 1 << 2;
    pub const LEFT: u8 = 1 << 3;
    pub const ALL: u8 = TOP | RIGHT | BOTTOM | LEFT;
}

/// A frame around a widget (`ui.Border`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WBorder {
    pub edges: u8,
    pub kind: WBorderType,
    pub title: Option<WLine>,
    pub style: WStyle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WConstraint {
    Min(u16),
    Max(u16),
    Length(u16),
    Percentage(u16),
    Ratio(u32, u32),
    Fill(u16),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WDirection {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WParagraph {
    pub text: WText,
    pub wrap: WWrap,
    pub scroll: u16,
    pub block: Option<WBorder>,
    pub style: WStyle,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WList {
    pub items: Vec<WText>,
    pub block: Option<WBorder>,
    pub style: WStyle,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WRow {
    pub cells: Vec<WText>,
    pub height: u16,
    pub style: WStyle,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WTable {
    pub header: Option<WRow>,
    pub rows: Vec<WRow>,
    /// Column widths; empty means equal columns.
    pub widths: Vec<WConstraint>,
    pub spacing: u16,
    pub block: Option<WBorder>,
    pub style: WStyle,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WGauge {
    pub ratio: f64,
    pub label: Option<WSpan>,
    pub block: Option<WBorder>,
    pub style: WStyle,
    pub gauge_style: WStyle,
}

/// Children laid out side by side or stacked, sized by `constraints`
/// (one per child; missing ones share the rest).
#[derive(Debug, Clone)]
pub struct WLayout {
    pub direction: WDirection,
    pub constraints: Vec<WConstraint>,
    pub margin: u16,
    pub children: Vec<Renderable>,
}

/// An image file drawn with the quick view's image renderer; `image` is
/// decoded by the host after `peek` returned (never by the plugin).
#[derive(Debug, Clone)]
pub struct PluginImage {
    pub path: PathBuf,
    pub image: Option<Arc<image::DynamicImage>>,
}

#[derive(Debug, Clone)]
pub enum Renderable {
    Paragraph(WParagraph),
    List(WList),
    Table(WTable),
    Gauge(WGauge),
    Layout(WLayout),
    Image(PluginImage),
    /// A widget drawn in `rect` of its area (`widget:area(rect)`).
    Placed(WRect, Box<Renderable>),
    /// Widgets drawn one over the other (`peek` returning a list).
    Stack(Vec<Renderable>),
    Bar(WBar),
    /// Clears its area (`ui.Clear`), e.g. under a placed popup.
    Clear,
    Fill(WFill),
}

/// Most widgets, spans and cells one `Renderable` may hold.
pub const MAX_RENDERABLE_NODES: usize = 100_000;
/// Most text bytes one `Renderable` may hold.
pub const MAX_RENDERABLE_BYTES: usize = 8 * 1024 * 1024;

impl Renderable {
    /// A plain paragraph of `text`.
    pub fn text(text: WText) -> Self {
        Self::Paragraph(WParagraph {
            text,
            wrap: WWrap::No,
            scroll: 0,
            block: None,
            style: WStyle::default(),
        })
    }

    /// `(nodes, text bytes)` held, to keep what a plugin hands over bounded.
    pub fn weight(&self) -> (usize, usize) {
        let texts = |texts: &mut dyn Iterator<Item = &WText>| {
            texts.fold((0, 0), |(n, b), t| (n + t.spans(), b + t.bytes()))
        };
        match self {
            Self::Paragraph(p) => {
                let (n, b) = texts(&mut std::iter::once(&p.text));
                (n + 1, b)
            }
            Self::List(l) => {
                let (n, b) = texts(&mut l.items.iter());
                (n + 1, b)
            }
            Self::Table(t) => {
                let mut cells = t.header.iter().chain(&t.rows).flat_map(|r| &r.cells);
                let (n, b) = texts(&mut cells);
                (n + t.rows.len() + 1, b)
            }
            Self::Gauge(g) => (1, g.label.as_ref().map_or(0, |l| l.content.len())),
            Self::Layout(l) => Self::weight_of(&l.children),
            Self::Stack(children) => Self::weight_of(children),
            Self::Placed(_, child) => {
                let (n, b) = child.weight();
                (n + 1, b)
            }
            Self::Bar(bar) => (1, bar.symbol.as_ref().map_or(0, String::len)),
            Self::Fill(fill) => (1, fill.symbol.len()),
            Self::Image(_) | Self::Clear => (1, 0),
        }
    }

    /// Weight of a container of `children` (itself counts as one node).
    fn weight_of(children: &[Renderable]) -> (usize, usize) {
        children
            .iter()
            .map(Self::weight)
            .fold((1, 0), |(n, b), (cn, cb)| (n + cn, b + cb))
    }

    /// Whether this tree stays within [`MAX_RENDERABLE_NODES`] and
    /// [`MAX_RENDERABLE_BYTES`].
    pub fn within_limits(&self) -> bool {
        let (nodes, bytes) = self.weight();
        nodes <= MAX_RENDERABLE_NODES && bytes <= MAX_RENDERABLE_BYTES
    }

    /// Images of the tree, for the host to decode before showing it.
    pub fn images_mut(&mut self) -> Vec<&mut PluginImage> {
        match self {
            Self::Image(image) => vec![image],
            Self::Layout(WLayout { children, .. }) | Self::Stack(children) => {
                children.iter_mut().flat_map(Self::images_mut).collect()
            }
            Self::Placed(_, child) => child.images_mut(),
            _ => Vec::new(),
        }
    }

    /// Lines of content, for the quick view scrollbar (`None` when the
    /// widget does not scroll as text).
    pub fn content_len(&self) -> Option<usize> {
        match self {
            Self::Paragraph(p) => Some(p.text.height()),
            Self::List(l) => Some(l.items.len()),
            Self::Table(t) => Some(t.rows.len()),
            _ => None,
        }
    }
}

/// A plugin previewer's output in the quick view: the widget, which plugin
/// drew it, at which `skip` and whether that plugin scrolls by itself
/// (`seek`), in which case PgUp / PgDn ask the plugin instead of
/// scrolling the widget.
#[derive(Debug, Clone)]
pub struct PluginPreview {
    pub widget: Renderable,
    pub plugin: String,
    pub skip: usize,
    pub seekable: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weight_counts_nested_widgets_and_text() {
        let para = Renderable::text(WText::raw("hello\nworld"));
        assert_eq!(para.weight(), (3, 10));
        let layout = Renderable::Layout(WLayout {
            direction: WDirection::Vertical,
            constraints: Vec::new(),
            margin: 0,
            children: vec![para.clone(), para],
        });
        assert_eq!(layout.weight(), (7, 20));
        assert!(layout.within_limits());
        let huge = Renderable::text(WText::raw(&"x".repeat(MAX_RENDERABLE_BYTES + 1)));
        assert!(!huge.within_limits());
    }

    #[test]
    fn images_are_found_inside_layouts() {
        let image = Renderable::Image(PluginImage {
            path: PathBuf::from("a.png"),
            image: None,
        });
        let mut layout = Renderable::Layout(WLayout {
            direction: WDirection::Horizontal,
            constraints: Vec::new(),
            margin: 0,
            children: vec![image, Renderable::text(WText::raw("x"))],
        });
        assert_eq!(layout.images_mut().len(), 1);
        assert_eq!(layout.content_len(), None);
    }
}
