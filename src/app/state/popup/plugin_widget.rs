/// Plugin-drawn widget stored on quick-view (and similar) overlays.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum PluginWidget {
    Paragraph(String),
    Gauge {
        ratio: f64,
        label: String,
    },
    List(Vec<String>),
    Table {
        headers: Vec<String>,
        rows: Vec<Vec<String>>,
    },
    Span {
        text: String,
        style: String,
    },
    Line(Vec<PluginWidget>),
}

/// A plugin previewer's output in the quick view: the widget, which plugin
/// drew it, at which `skip` and whether that plugin scrolls by itself
/// (`seek`), in which case PgUp / PgDn ask the plugin instead of
/// scrolling the widget.
#[derive(Debug, Clone)]
pub struct PluginPreview {
    pub widget: PluginWidget,
    pub plugin: String,
    pub skip: usize,
    pub seekable: bool,
}
