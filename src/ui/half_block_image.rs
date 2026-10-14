//! Draws an image with Unicode half blocks (two pixels per cell), fitted to
//! an area and centered (quick view and F3 viewer).

use image::GenericImageView;
use ratatui::{Frame, layout::Rect, style::Color};

/// Draws `img` fitted whole inside `area` (so there is nothing to scroll).
/// `bg` fills the lower half of the last row of odd-height images and shows
/// through transparent pixels.
pub fn draw(f: &mut Frame, area: Rect, img: &image::DynamicImage, bg: Color) {
    if area.width == 0 || area.height == 0 {
        return;
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
    let rgb = |x: u32, y: u32| over(resized.get_pixel(x, y).0, bg);

    let buf = f.buffer_mut();
    for row in 0..rows {
        let y = start_y + row as u16;
        for col in 0..dw {
            let x = start_x + col as u16;
            if x >= area.x + area.width {
                continue;
            }
            let Some(cell) = buf.cell_mut((x, y)) else {
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
}

/// `rgba` composited over `bg` (an RGB background; any other background
/// shows only where the pixel is mostly transparent).
fn over([r, g, b, a]: [u8; 4], bg: Color) -> Color {
    let blend = |c: u8, under: u8| {
        ((u16::from(c) * u16::from(a) + u16::from(under) * u16::from(255 - a)) / 255) as u8
    };
    match bg {
        _ if a == 255 => Color::Rgb(r, g, b),
        Color::Rgb(br, bg_, bb) => Color::Rgb(blend(r, br), blend(g, bg_), blend(b, bb)),
        _ if a < 128 => bg,
        _ => Color::Rgb(r, g, b),
    }
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
    use super::{Color, fit, over};

    #[test]
    fn transparent_pixels_show_the_background() {
        let bg = Color::Rgb(10, 20, 30);
        assert_eq!(over([200, 0, 0, 255], bg), Color::Rgb(200, 0, 0));
        assert_eq!(over([200, 0, 0, 0], bg), bg);
        assert_eq!(over([0, 0, 0, 0], Color::Reset), Color::Reset);
        assert_eq!(
            over([255, 255, 255, 255], Color::Reset),
            Color::Rgb(255, 255, 255)
        );
    }

    #[test]
    fn fits_width_or_height() {
        assert_eq!(fit(100, 50, 40, 40), (40, 20));
        assert_eq!(fit(50, 100, 40, 40), (20, 40));
        assert_eq!(fit(1, 1000, 40, 2), (1, 2));
    }
}
