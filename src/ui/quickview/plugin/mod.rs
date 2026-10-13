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
    draw(f, inner, widget, (base, theme), scroll);
    if let Some(content_len) = widget.content_len() {
        render_scrollbar(f, area, theme, (content_len, scroll), scrollbar);
    }
}

/// Inner area of a widget's own border (if any), after drawing it.
fn framed(
    f: &mut Frame,
    area: Rect,
    border: Option<&crate::app::state::popup::plugin_widget::WBorder>,
) -> Rect {
    match border {
        Some(border) => {
            let block = convert::block(border);
            let inner = block.inner(area);
            f.render_widget(block, area);
            inner
        }
        None => area,
    }
}

fn draw(
    f: &mut Frame,
    area: Rect,
    widget: &Renderable,
    (base, theme): (Style, &ResolvedTheme),
    scroll: usize,
) {
    match widget {
        Renderable::Paragraph(p) => draw_paragraph(f, area, p, base, scroll),
        Renderable::List(l) => draw_list(f, area, l, base, scroll),
        Renderable::Table(table) => draw_table(f, area, table, base, scroll),
        Renderable::Gauge(g) => draw_gauge(f, area, g, base),
        Renderable::Layout(l) => draw_layout(f, area, l, (base, theme)),
        Renderable::Image(image) => draw_image(f, area, image, (base, theme), scroll),
    }
}

fn draw_paragraph(f: &mut Frame, area: Rect, p: &WParagraph, base: Style, scroll: usize) {
    let area = framed(f, area, p.block.as_ref());
    let offset = (p.scroll as usize)
        .saturating_add(scroll)
        .min(u16::MAX as usize) as u16;
    let mut para = Paragraph::new(convert::text(&p.text))
        .style(base.patch(convert::style(&p.style)))
        .scroll((offset, 0));
    if p.wrap != WWrap::No {
        para = para.wrap(Wrap {
            trim: p.wrap == WWrap::Trim,
        });
    }
    f.render_widget(para, area);
}

fn draw_list(f: &mut Frame, area: Rect, l: &WList, base: Style, scroll: usize) {
    let area = framed(f, area, l.block.as_ref());
    let items: Vec<ListItem> = l
        .items
        .iter()
        .skip(scroll)
        .map(|item| ListItem::new(convert::text(item)))
        .collect();
    f.render_widget(
        List::new(items).style(base.patch(convert::style(&l.style))),
        area,
    );
}

fn draw_table(f: &mut Frame, area: Rect, table: &WTable, base: Style, scroll: usize) {
    let area = framed(f, area, table.block.as_ref());
    let row = |r: &crate::app::state::popup::plugin_widget::WRow| {
        Row::new(r.cells.iter().map(convert::text))
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
        .style(base.patch(convert::style(&table.style)));
    if let Some(header) = &table.header {
        widget = widget
            .header(row(header).style(convert::style(&header.style).add_modifier(Modifier::BOLD)));
    }
    f.render_widget(widget, area);
}

fn draw_gauge(f: &mut Frame, area: Rect, g: &WGauge, base: Style) {
    let area = framed(f, area, g.block.as_ref());
    let mut gauge = Gauge::default()
        .ratio(g.ratio.clamp(0.0, 1.0))
        .style(base.patch(convert::style(&g.style)))
        .gauge_style(convert::style(&g.gauge_style));
    if let Some(label) = &g.label {
        gauge = gauge.label(convert::span(label));
    }
    f.render_widget(gauge, area);
}

fn draw_layout(f: &mut Frame, area: Rect, l: &WLayout, cx: (Style, &ResolvedTheme)) {
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

fn draw_image(
    f: &mut Frame,
    area: Rect,
    image: &PluginImage,
    (base, theme): (Style, &ResolvedTheme),
    scroll: usize,
) {
    match &image.image {
        Some(img) => super::render_quick_view_image(f, area, img, Block::default(), theme, scroll),
        None => f.render_widget(
            Paragraph::new(t("quickview_plugin_image_error")).style(base),
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
