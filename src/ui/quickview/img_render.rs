use crate::ui::theme_apply::parse_color;
use image::GenericImageView;
use ratatui::{Frame, layout::Rect, widgets::Block};

pub fn render_quick_view_image(
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
    let rows = (dh as usize).div_ceil(2);

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

            if let Some(cell) = buf.cell_mut((target_x, target_y as u16)) {
                if py_bottom < dh as usize {
                    let pixel_bottom = resized.get_pixel(r_x as u32, py_bottom as u32);
                    let color_bottom = ratatui::style::Color::Rgb(
                        pixel_bottom[0],
                        pixel_bottom[1],
                        pixel_bottom[2],
                    );
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
}
