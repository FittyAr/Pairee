//! Draws an image with Unicode half blocks (two pixels per cell), fitted to
//! an area and centered (quick view and F3 viewer).

use image::GenericImageView;
use ratatui::{Frame, layout::Rect, style::Color};

/// Draws `img` fitted inside `area`, scrolled down by `scroll` cell rows.
/// `bg` fills the lower half of the last row of odd-height images. Returns
/// the image height in cell rows.
pub fn draw(
    f: &mut Frame,
    area: Rect,
    img: &image::DynamicImage,
    scroll: usize,
    bg: Color,
) -> usize {
    if area.width == 0 || area.height == 0 {
        return 0;
    }
    let (dw, dh) = fit(
        img.width(),
        img.height(),
        area.width as u32,
        area.height as u32 * 2,
    );
    let resized = img.resize_exact(dw, dh, image::imageops::FilterType::Nearest);
    let rows = (dh as usize).div_ceil(2);
    let start_x = area.x + (area.width - dw as u16) / 2;
    let start_y = area.y + (area.height - rows as u16) / 2;
    let rgb = |x: u32, y: u32| {
        let p = resized.get_pixel(x, y);
        Color::Rgb(p[0], p[1], p[2])
    };

    let buf = f.buffer_mut();
    for row in 0..rows {
        let y = start_y as i64 + row as i64 - scroll as i64;
        if y < area.y as i64 || y >= (area.y + area.height) as i64 {
            continue;
        }
        for col in 0..dw {
            let x = start_x + col as u16;
            if x >= area.x + area.width {
                continue;
            }
            let Some(cell) = buf.cell_mut((x, y as u16)) else {
                continue;
            };
            let top = rgb(col, 2 * row as u32);
            if 2 * row + 1 < dh as usize {
                cell.set_char('▄');
                cell.set_fg(rgb(col, 2 * row as u32 + 1));
                cell.set_bg(top);
            } else {
                cell.set_char('▀');
                cell.set_fg(top);
                cell.set_bg(bg);
            }
        }
    }
    rows
}

/// Largest size with the image's aspect ratio inside `max_w` × `max_h`
/// pixels (at least 1×1).
fn fit(img_w: u32, img_h: u32, max_w: u32, max_h: u32) -> (u32, u32) {
    let ratio = img_w as f64 / img_h as f64;
    let (w, h) = if (max_w as f64 / ratio) <= max_h as f64 {
        (max_w, (max_w as f64 / ratio) as u32)
    } else {
        ((max_h as f64 * ratio) as u32, max_h)
    };
    (w.max(1), h.max(1))
}

#[cfg(test)]
mod tests {
    use super::fit;

    #[test]
    fn fits_width_or_height() {
        assert_eq!(fit(100, 50, 40, 40), (40, 20));
        assert_eq!(fit(50, 100, 40, 40), (20, 40));
        assert_eq!(fit(1, 1000, 40, 2), (1, 2));
    }
}
