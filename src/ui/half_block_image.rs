//! Draws an image with Unicode half blocks (two pixels per cell), fitted to
//! an area and centered (quick view and F3 viewer).

use image::{DynamicImage, GenericImageView};
use ratatui::{Frame, layout::Rect, style::Color};
use std::cell::RefCell;
use std::sync::{Arc, Weak};

/// Draws `img` fitted whole inside `area` (so there is nothing to scroll).
/// `bg` fills the lower half of the last row of odd-height images and shows
/// through transparent pixels. The cells are computed once per image, size
/// and background (see [`cells`]), so redraws do not resize the image again.
pub fn draw(f: &mut Frame, area: Rect, img: &Arc<DynamicImage>, bg: Color) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let (dw, dh) = fit(
        img.width(),
        img.height(),
        area.width as u32,
        area.height as u32 * 2,
    );
    let image = cells(img, (dw, dh), bg);
    let rows = image.rows();
    let start_x = area.x + (area.width - dw as u16) / 2;
    let start_y = area.y + (area.height - rows as u16) / 2;

    let buf = f.buffer_mut();
    for (row, line) in image.cells.chunks(dw as usize).enumerate() {
        let y = start_y + row as u16;
        for (col, &(symbol, fg, cell_bg)) in line.iter().enumerate() {
            let x = start_x + col as u16;
            if x >= area.x + area.width {
                continue;
            }
            if let Some(cell) = buf.cell_mut((x, y)) {
                cell.set_char(symbol).set_fg(fg).set_bg(cell_bg);
            }
        }
    }
}

/// An image resized to `width` × `height` pixels as half-block cells (row by
/// row: character, foreground, background).
struct HalfBlocks {
    width: u32,
    cells: Vec<(char, Color, Color)>,
}

impl HalfBlocks {
    fn rows(&self) -> usize {
        self.cells.len() / self.width as usize
    }

    fn new(img: &DynamicImage, (dw, dh): (u32, u32), bg: Color) -> Self {
        let resized = img.resize_exact(dw, dh, image::imageops::FilterType::Nearest);
        let rgb = |x: u32, y: u32| over(resized.get_pixel(x, y).0, bg);
        let rows = dh.div_ceil(2);
        let mut cells = Vec::with_capacity((rows * dw) as usize);
        for row in 0..rows {
            for col in 0..dw {
                let top = rgb(col, 2 * row);
                cells.push(if 2 * row + 1 < dh {
                    ('▄', rgb(col, 2 * row + 1), top)
                } else {
                    ('▀', top, bg)
                });
            }
        }
        Self { width: dw, cells }
    }
}

/// Images whose cells were computed recently. A [`Weak`] keeps the
/// allocation of the image alive, so its address cannot be reused by
/// another image while the entry exists.
type CacheEntry = (Weak<DynamicImage>, (u32, u32), Color, Arc<HalfBlocks>);

/// Quick views of both panels, a plugin preview and the viewer can show
/// pictures at the same time.
const CACHED_IMAGES: usize = 4;

thread_local! {
    static CACHE: RefCell<Vec<CacheEntry>> = const { RefCell::new(Vec::new()) };
}

/// The cells of `img` at `size` over `bg`: from the cache, or computed and
/// cached (most recent first; dropped images and the oldest entries go).
fn cells(img: &Arc<DynamicImage>, size: (u32, u32), bg: Color) -> Arc<HalfBlocks> {
    CACHE.with_borrow_mut(|cache| {
        let id = Arc::downgrade(img);
        cache.retain(|(image, ..)| image.strong_count() > 0);
        let found = cache
            .iter()
            .position(|(image, s, b, _)| Weak::ptr_eq(image, &id) && *s == size && *b == bg);
        let entry = match found {
            Some(i) => cache.remove(i),
            None => (id, size, bg, Arc::new(HalfBlocks::new(img, size, bg))),
        };
        let blocks = Arc::clone(&entry.3);
        cache.insert(0, entry);
        cache.truncate(CACHED_IMAGES);
        blocks
    })
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
    use super::{Color, cells, fit, over};
    use std::sync::Arc;

    fn picture(width: u32, height: u32) -> Arc<image::DynamicImage> {
        Arc::new(image::DynamicImage::new_rgba8(width, height))
    }

    #[test]
    fn redraws_reuse_the_resized_cells() {
        let img = picture(8, 8);
        let first = cells(&img, (4, 4), Color::Reset);
        assert_eq!((first.width, first.rows()), (4, 2));
        assert!(Arc::ptr_eq(&first, &cells(&img, (4, 4), Color::Reset)));
        assert!(!Arc::ptr_eq(&first, &cells(&img, (2, 2), Color::Reset)));
        assert!(!Arc::ptr_eq(&first, &cells(&img, (4, 4), Color::Black)));
        let other = picture(8, 8);
        assert!(!Arc::ptr_eq(&first, &cells(&other, (4, 4), Color::Reset)));
    }

    #[test]
    fn odd_heights_end_in_an_upper_half_block() {
        let blocks = cells(&picture(2, 3), (2, 3), Color::Blue);
        let symbols: Vec<char> = blocks.cells.iter().map(|c| c.0).collect();
        assert_eq!(symbols, ['▄', '▄', '▀', '▀']);
        assert_eq!(blocks.cells[3].2, Color::Blue);
    }

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
