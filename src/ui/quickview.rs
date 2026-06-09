use crate::config::localization::t;
use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};

/// Renders a quick-view panel showing the text content of a file.
/// Called when `state.quick_view_active` is true; renders into the passive panel area.
///
/// - Scrolls vertically via `scroll` offset.
/// - Non-UTF-8 files show a binary notice.
pub fn draw_quick_view(
    f: &mut Frame,
    area: Rect,
    path: &std::path::Path,
    content: &[String],
    scroll: usize,
    theme: &crate::config::theme::Theme,
) {
    let file_name = path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "?".to_string());
    
    let title = t("quickview_title")
        .replacen("{}", &file_name, 1);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(parse_color(&theme.popup_border)))
        .title(ratatui::widgets::block::Title::from(Span::styled(
            title,
            Style::default()
                .fg(parse_color(&theme.header_fg))
                .add_modifier(Modifier::BOLD),
        )))
        .style(Style::default().bg(parse_color(&theme.panel_bg)));

    let visible_height = area.height.saturating_sub(2) as usize;
    let lines: Vec<Line> = content
        .iter()
        .skip(scroll)
        .take(visible_height)
        .map(|l| Line::from(Span::raw(l.clone())))
        .collect();

    let para = Paragraph::new(lines)
        .block(block)
        .style(Style::default().fg(parse_color(&theme.panel_fg)))
        .wrap(Wrap { trim: false });

    f.render_widget(para, area);
}

pub fn load_quick_view_content(path: &std::path::Path) -> Vec<String> {
    let format = crate::fs::archive::detect_format(path);
    match format {
        crate::fs::archive::ArchiveFormat::Zip
        | crate::fs::archive::ArchiveFormat::TarGz
        | crate::fs::archive::ArchiveFormat::SevenZ => {
            match crate::fs::archive::list_archive_files(path) {
                Ok(files) => {
                    let format_name = match format {
                        crate::fs::archive::ArchiveFormat::Zip => "ZIP",
                        crate::fs::archive::ArchiveFormat::TarGz => "TarGz",
                        crate::fs::archive::ArchiveFormat::SevenZ => "7Z",
                        _ => "Archive",
                    };
                    let archive_name = path.file_name().unwrap_or_default().to_string_lossy();
                    let files_count = files.len().to_string();
                    let mut lines = vec![
                        t("quickview_archive").replacen("{}", &archive_name, 1),
                        t("quickview_format").replacen("{}", format_name, 1),
                        t("quickview_files").replacen("{}", &files_count, 1),
                        "────────────────────────────────────────".to_string(),
                    ];
                    for f in files {
                        lines.push(f);
                    }
                    lines
                }
                Err(e) => {
                    let err_str = e.to_string();
                    vec![t("quickview_error").replacen("{}", &err_str, 1)]
                }
            }
        }
        _ => match std::fs::read_to_string(path) {
            Ok(text) => text.lines().map(|l| l.to_string()).collect(),
            Err(_) => vec![t("quickview_binary_no_preview")],
        },
    }
}
