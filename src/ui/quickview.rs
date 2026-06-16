use crate::config::localization::t;
use crate::ui::theme_apply::parse_color;
use image::GenericImageView;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};

/// Renders a quick-view panel showing the text or image content of a file.
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
    image_data: &Option<image::DynamicImage>,
) {
    let file_name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "?".to_string());

    let title = t("quickview_title").replacen("{}", &file_name, 1);

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

    if let Some(img) = image_data {
        render_quick_view_image(f, area, img, block, theme, scroll);
    } else {
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
}

fn render_quick_view_image(
    f: &mut Frame,
    area: Rect,
    img: &image::DynamicImage,
    block: Block,
    theme: &crate::config::theme::Theme,
    scroll_offset: usize,
) {
    let inner_area = block.inner(area);
    let inner_w = inner_area.width;
    let inner_h = inner_area.height;

    if inner_w == 0 || inner_h == 0 {
        return;
    }

    let img_w = img.width();
    let img_h = img.height();

    let canvas_w = inner_w as u32;
    let canvas_h = inner_h as u32 * 2;

    let r = img_w as f64 / img_h as f64;
    let (mut dw, mut dh) = if (canvas_w as f64 / r) <= canvas_h as f64 {
        let w = canvas_w;
        let h = (w as f64 / r) as u32;
        (w, h)
    } else {
        let h = canvas_h;
        let w = (h as f64 * r) as u32;
        (w, h)
    };

    if dw == 0 {
        dw = 1;
    }
    if dh == 0 {
        dh = 1;
    }

    let resized = img.resize_exact(dw, dh, image::imageops::FilterType::Nearest);

    let cols = dw as usize;
    let rows = (dh as usize + 1) / 2;

    let start_x = inner_area.x + ((inner_w - dw as u16) / 2);
    let start_y = inner_area.y + ((inner_h - rows as u16) / 2);

    f.render_widget(block, area);

    let buf = f.buffer_mut();

    for r_y in 0..rows {
        let target_y = start_y as i32 + r_y as i32 - scroll_offset as i32;
        if target_y < inner_area.y as i32 || target_y >= (inner_area.y + inner_h) as i32 {
            continue;
        }

        for r_x in 0..cols {
            let target_x = start_x + r_x as u16;
            if target_x >= inner_area.x + inner_w {
                continue;
            }

            let py_top = 2 * r_y;
            let py_bottom = 2 * r_y + 1;

            let pixel_top = resized.get_pixel(r_x as u32, py_top as u32);
            let color_top = ratatui::style::Color::Rgb(pixel_top[0], pixel_top[1], pixel_top[2]);

            let cell = buf.get_mut(target_x, target_y as u16);
            if py_bottom < dh as usize {
                let pixel_bottom = resized.get_pixel(r_x as u32, py_bottom as u32);
                let color_bottom =
                    ratatui::style::Color::Rgb(pixel_bottom[0], pixel_bottom[1], pixel_bottom[2]);
                cell.set_char('▄');
                cell.set_fg(color_bottom);
                cell.set_bg(color_top);
            } else {
                cell.set_char('▀');
                cell.set_fg(color_top);
                cell.set_bg(parse_color(&theme.panel_bg));
            }
        }
    }
}

pub fn load_quick_view_content(path: &std::path::Path) -> Vec<String> {
    if path.is_dir() {
        let dir_name = path.file_name().unwrap_or_default().to_string_lossy();
        let mut lines = vec![
            t("quickview_folder").replacen("{}", &dir_name, 1),
            "────────────────────────────────────────".to_string(),
        ];
        if let Ok(entries) = std::fs::read_dir(path) {
            let mut entries_vec: Vec<_> = entries.flatten().collect();
            entries_vec.sort_by(|a, b| {
                let a_dir = a.file_type().map(|t| t.is_dir()).unwrap_or(false);
                let b_dir = b.file_type().map(|t| t.is_dir()).unwrap_or(false);
                match (a_dir, b_dir) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => {
                        let a_name = a.file_name().to_string_lossy().to_lowercase();
                        let b_name = b.file_name().to_string_lossy().to_lowercase();
                        a_name.cmp(&b_name)
                    }
                }
            });
            for entry in entries_vec {
                let name = entry.file_name().to_string_lossy().into_owned();
                let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
                if is_dir {
                    lines.push(format!("{}/", name));
                } else {
                    lines.push(name);
                }
            }
        }
        return lines;
    }

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
