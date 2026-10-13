//! Plugin widgets drawn into a test terminal.

use super::*;
use crate::app::state::popup::plugin_widget::{
    WConstraint, WDirection, WGauge, WLayout, WRow, WSpan, WStyle, WTable, WText,
};
use ratatui::{Terminal, backend::TestBackend};

fn screen(widget: &Renderable, width: u16, height: u16) -> String {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    let theme = ResolvedTheme::default();
    terminal
        .draw(|f| {
            let area = f.area();
            render_plugin_widget(f, area, widget, Block::bordered(), &theme, 0, None);
        })
        .unwrap();
    let buffer = terminal.backend().buffer();
    (0..height)
        .map(|y| {
            (0..width)
                .map(|x| buffer[(x, y)].symbol().to_string())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn table() -> Renderable {
    let row = |cells: &[&str]| WRow {
        cells: cells.iter().map(|c| WText::raw(c)).collect(),
        height: 1,
        style: WStyle::default(),
    };
    Renderable::Table(WTable {
        header: Some(row(&["Name", "Size"])),
        rows: vec![row(&["a.txt", "12"])],
        widths: vec![WConstraint::Length(8), WConstraint::Fill(1)],
        spacing: 1,
        block: None,
        style: WStyle::default(),
    })
}

#[test]
fn a_layout_draws_each_child_in_its_area() {
    let gauge = Renderable::Gauge(WGauge {
        ratio: 0.5,
        label: Some(WSpan {
            content: "half".into(),
            style: WStyle::default(),
        }),
        block: None,
        style: WStyle::default(),
        gauge_style: WStyle::default(),
    });
    let layout = Renderable::Layout(WLayout {
        direction: WDirection::Vertical,
        constraints: vec![
            WConstraint::Length(1),
            WConstraint::Length(2),
            WConstraint::Fill(1),
        ],
        margin: 0,
        children: vec![Renderable::text(WText::raw("title line")), table(), gauge],
    });
    let text = screen(&layout, 30, 8);
    let lines: Vec<&str> = text.lines().collect();
    assert!(lines[1].contains("title line"), "{text}");
    assert!(
        lines[2].contains("Name") && lines[2].contains("Size"),
        "{text}"
    );
    assert!(lines[3].contains("a.txt"), "{text}");
    assert!(text.contains("half"), "{text}");
}

#[test]
fn a_missing_image_says_so() {
    let image = Renderable::Image(PluginImage {
        path: "x.png".into(),
        image: None,
    });
    let text = screen(&image, 50, 4);
    assert!(
        text.contains(&t("quickview_plugin_image_error")[..10]),
        "{text}"
    );
}
