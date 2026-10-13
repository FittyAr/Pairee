//! Draws a plugin previewer's [`Renderable`] in the quick view: the quick
//! view frame, then the widget tree inside it (layouts split the area,
//! images go through the quick view's image renderer). `scroll` is the
//! quick view's own scroll offset, applied to a top-level paragraph, list
//! or table.

mod convert;

use crate::app::state::popup::plugin_widget::{
    PluginImage, Renderable, WGauge, WLayout, WList, WParagraph, WTable, WWrap,
};
use crate::config::localization::t;
use crate::ui::resolved_theme::ResolvedTheme;
use crate::ui::scrollbar::{self, ScrollTarget, ScrollTargetId, ScrollView};
use crate::ui::scrollbar::{ScrollbarSurface, ScrollbarUiState};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::{Modifier, Style},
    widgets::{Block, Gauge, List, ListItem, Paragraph, Row, Table, Wrap},
};

pub fn render_plugin_widget(
    f: &mut Frame,
    area: Rect,
    widget: &Renderable,
    block: Block,
    theme: &ResolvedTheme,
    scroll: usize,
    scrollbar: Option<&ScrollbarUiState>,
) {
    let inner = block.inner(area);
    f.render_widget(block, area);
    let base = Style::default().fg(theme.panel_fg);
    draw(f, inner, widget, Paint { base, theme }, scroll);
    if let Some(content_len) = widget.content_len() {
        render_scrollbar(f, area, theme, (content_len, scroll), scrollbar);
    }
}

/// What every widget is drawn with: the panel text style under it and the
/// theme (syntax colors of highlighted code).
#[derive(Clone, Copy)]
struct Paint<'a> {
    base: Style,
    theme: &'a ResolvedTheme,
}

/// Inner area of a widget's own border (if any), after drawing it.
fn framed(
    f: &mut Frame,
    area: Rect,
    border: Option<&crate::app::state::popup::plugin_widget::WBorder>,
    theme: &ResolvedTheme,
) -> Rect {
    match border {
        Some(border) => {
            let block = convert::block(border, theme);
            let inner = block.inner(area);
            f.render_widget(block, area);
            inner
        }
        None => area,
    }
}

fn draw(f: &mut Frame, area: Rect, widget: &Renderable, cx: Paint, scroll: usize) {
    match widget {
        Renderable::Paragraph(p) => draw_paragraph(f, area, p, cx, scroll),
        Renderable::List(l) => draw_list(f, area, l, cx, scroll),
        Renderable::Table(table) => draw_table(f, area, table, cx, scroll),
        Renderable::Gauge(g) => draw_gauge(f, area, g, cx),
        Renderable::Layout(l) => draw_layout(f, area, l, cx),
        Renderable::Image(image) => draw_image(f, area, image, cx, scroll),
    }
}

fn draw_paragraph(f: &mut Frame, area: Rect, p: &WParagraph, cx: Paint, scroll: usize) {
    let area = framed(f, area, p.block.as_ref(), cx.theme);
    let offset = (p.scroll as usize)
        .saturating_add(scroll)
        .min(u16::MAX as usize) as u16;
    let mut para = Paragraph::new(convert::text(&p.text, cx.theme))
        .style(cx.base.patch(convert::style(&p.style)))
        .scroll((offset, 0));
    if p.wrap != WWrap::No {
        para = para.wrap(Wrap {
            trim: p.wrap == WWrap::Trim,
        });
    }
    f.render_widget(para, area);
}

fn draw_list(f: &mut Frame, area: Rect, l: &WList, cx: Paint, scroll: usize) {
    let area = framed(f, area, l.block.as_ref(), cx.theme);
    let items: Vec<ListItem> = l
        .items
        .iter()
        .skip(scroll)
        .map(|item| ListItem::new(convert::text(item, cx.theme)))
        .collect();
    f.render_widget(
        List::new(items).style(cx.base.patch(convert::style(&l.style))),
        area,
    );
}

fn draw_table(f: &mut Frame, area: Rect, table: &WTable, cx: Paint, scroll: usize) {
    let area = framed(f, area, table.block.as_ref(), cx.theme);
    let row = |r: &crate::app::state::popup::plugin_widget::WRow| {
        Row::new(r.cells.iter().map(|c| convert::text(c, cx.theme)))
            .height(r.height.max(1))
            .style(convert::style(&r.style))
    };
    let columns = table
        .header
        .iter()
        .chain(&table.rows)
        .map(|r| r.cells.len())
        .max()
        .unwrap_or(0);
    let widths: Vec<Constraint> = if table.widths.is_empty() {
        vec![Constraint::Fill(1); columns]
    } else {
        table.widths.iter().map(convert::constraint).collect()
    };
    let mut widget = Table::new(table.rows.iter().skip(scroll).map(row), widths)
        .column_spacing(table.spacing)
        .style(cx.base.patch(convert::style(&table.style)));
    if let Some(header) = &table.header {
        widget = widget
            .header(row(header).style(convert::style(&header.style).add_modifier(Modifier::BOLD)));
    }
    f.render_widget(widget, area);
}

fn draw_gauge(f: &mut Frame, area: Rect, g: &WGauge, cx: Paint) {
    let area = framed(f, area, g.block.as_ref(), cx.theme);
    let mut gauge = Gauge::default()
        .ratio(g.ratio.clamp(0.0, 1.0))
        .style(cx.base.patch(convert::style(&g.style)))
        .gauge_style(convert::style(&g.gauge_style));
    if let Some(label) = &g.label {
        gauge = gauge.label(convert::span(label, cx.theme));
    }
    f.render_widget(gauge, area);
}

fn draw_layout(f: &mut Frame, area: Rect, l: &WLayout, cx: Paint) {
    let area = area.inner(Margin::new(l.margin, l.margin));
    let count = l.children.len();
    let constraints: Vec<Constraint> = (0..count)
        .map(|i| {
            l.constraints
                .get(i)
                .map_or(Constraint::Fill(1), convert::constraint)
        })
        .collect();
    let direction = match l.direction {
        crate::app::state::popup::plugin_widget::WDirection::Horizontal => Direction::Horizontal,
        crate::app::state::popup::plugin_widget::WDirection::Vertical => Direction::Vertical,
    };
    let areas = Layout::default()
        .direction(direction)
        .constraints(constraints)
        .split(area);
    for (child, child_area) in l.children.iter().zip(areas.iter()) {
        draw(f, *child_area, child, cx, 0);
    }
}

fn draw_image(f: &mut Frame, area: Rect, image: &PluginImage, cx: Paint, scroll: usize) {
    match &image.image {
        Some(img) => {
            super::render_quick_view_image(f, area, img, Block::default(), cx.theme, scroll)
        }
        None => f.render_widget(
            Paragraph::new(t("quickview_plugin_image_error")).style(cx.base),
            area,
        ),
    }
}

/// Quick View scrollbar inside the frame; `content` is (length, offset).
fn render_scrollbar(
    f: &mut Frame,
    area: Rect,
    theme: &ResolvedTheme,
    (content_len, offset): (usize, usize),
    scrollbar: Option<&ScrollbarUiState>,
) {
    scrollbar::render_vertical_inside_block(
        f,
        area,
        ScrollView {
            content_len,
            viewport_len: area.height.saturating_sub(2) as usize,
            offset,
        },
        theme,
        ScrollTarget {
            surface: ScrollbarSurface::Panel,
            hits: scrollbar,
            id: ScrollTargetId::QuickView,
        },
    );
}

#[cfg(test)]
mod tests;
