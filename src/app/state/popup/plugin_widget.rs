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
