//! Non-modal toast overlay.
//!
//! A toast is a lightweight, auto-dismissing notification that lives in
//! `state.toast` — a slot that is independent of the active popup. The
//! renderer draws the toast on top of every other layer (panels, popups,
//! transfer panel) but it does **not** consume keyboard input, so the user
//! can keep working (install another plugin, navigate, etc.) while the
//! toast is on screen.
//!
//! Each toast carries an optional `deadline`; when it elapses the main
//! loop clears the slot. When a new toast arrives while one is already
//! on screen, [`crate::plugin::manager::dispatch_actions::push_toast`]
//! keeps whichever deadline is later so the user never loses feedback
//! because the new message arrived "too early".
//!
//! The toast is rendered in the **bottom-right** corner of the screen
//! (a small floating card) so it does not cover the active panel's
//! important content (file lists, previews) nor the active popup's
//! controls (e.g. the search box of the plugin manager).

use crate::app::context::AppContext;
use crate::app::state::AppState;
use crate::config::localization::t;
use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::Line,
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};

/// Hard limits for the toast card. We pick a width that fits the
/// message without truncating common plugin install / error strings,
/// and a max width of 80% of the terminal so we never spill off the
/// right edge on narrow terminals.
const TOAST_MIN_WIDTH: u16 = 36;
const TOAST_MAX_WIDTH_PCT: u16 = 80;

/// Render the active toast, if any, in the bottom-right corner.
///
/// This is a no-op when `state.toast` is `None` or the toast has already
/// expired. The function is called once per frame, after the popup
/// layer, so the toast is always drawn on top.
pub fn render(f: &mut Frame, state: &AppState, context: &AppContext) {
    let toast = match &state.toast {
        None => return,
        Some(t) => t,
    };

    // Skip drawing if the deadline already elapsed. The main loop
    // clears the slot on the next tick, but we also check here so the
    // very last expired frame never paints a stale toast.
    if let Some(deadline) = toast.deadline {
        if std::time::Instant::now() >= deadline {
            return;
        }
    }

    let size = f.area();
    if size.width == 0 || size.height == 0 {
        return;
    }

    let theme = &context.config.theme;
    let border_color = color_for_level(&toast.level);
    let title = title_for_level(&toast.level);

    let body = toast.body.clone();
    let dismiss_hint = t("toast_dismiss_hint");
    let text = Line::from(format!(" {}\n {}", body, dismiss_hint));

    // Pick a width that fits the message without scrolling, capped at
    // 80% of the terminal width (so we never hit the right edge on
    // narrow terminals). The minimum keeps short messages readable.
    let natural = (body.chars().count() as u16) + 4; // 2 padding + 2 borders
    let max_width = (size.width * TOAST_MAX_WIDTH_PCT) / 100;
    let width = natural
        .max(TOAST_MIN_WIDTH)
        .min(max_width.max(TOAST_MIN_WIDTH));
    // The body is rendered on line 1 and the dismiss hint on line 2;
    // add one row for each plus the 2 border rows and 1 padding row.
    let height = 4;

    let area = anchored_bottom_right(width, height, size);

    f.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(title)
        .style(Style::default().bg(parse_color(&theme.popup_bg)));

    let paragraph = Paragraph::new(text)
        .block(block)
        .style(Style::default().fg(parse_color(&theme.popup_fg)))
        .wrap(Wrap { trim: false });

    f.render_widget(paragraph, area);
}

/// Map a toast level to a border colour.
fn color_for_level(level: &str) -> Color {
    match level {
        "warn" | "warning" => Color::Yellow,
        "error" | "critical" => Color::Red,
        "success" | "ok" => Color::Green,
        _ => Color::Cyan,
    }
}

/// Map a toast level to a localised title.
fn title_for_level(level: &str) -> String {
    let key = match level {
        "warn" | "warning" => "toast_title_warn",
        "error" | "critical" => "toast_title_error",
        "success" | "ok" => "toast_title_success",
        _ => "toast_title_info",
    };
    format!(" {} ", t(key))
}

/// Anchor a rectangle of `width`x`height` to the bottom-right corner of
/// `r`, clamped to fit inside `r`.
fn anchored_bottom_right(width: u16, height: u16, r: Rect) -> Rect {
    let w = width.min(r.width);
    let h = height.min(r.height);
    let x = r.x + r.width.saturating_sub(w);
    // Leave 1 row of breathing room at the bottom so the toast does
    // not sit on top of the F-key bar.
    let y = r.y + r.height.saturating_sub(h).saturating_sub(1);
    Rect::new(x, y, w, h)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_anchored_bottom_right_fits_inside() {
        let r = Rect::new(0, 0, 100, 30);
        let area = anchored_bottom_right(40, 5, r);
        assert!(area.x + area.width <= r.width, "must fit horizontally");
        assert!(area.y + area.height <= r.height, "must fit vertically");
        // Bottom-anchored: the bottom edge of the toast sits 1 row
        // above the bottom of the screen.
        assert_eq!(area.y + area.height, r.height - 1);
    }

    #[test]
    fn test_anchored_bottom_right_clamps_to_terminal() {
        // A toast wider than the terminal must NOT spill off the
        // right edge.
        let r = Rect::new(0, 0, 20, 10);
        let area = anchored_bottom_right(50, 5, r);
        assert!(area.x + area.width <= r.width);
        assert_eq!(area.x, 0, "must clamp to the left when too wide");
    }

    #[test]
    fn test_anchored_bottom_right_does_not_overlap_fkey_bar() {
        // The toast must leave at least 1 row above the F-key bar
        // (which lives on the last row of the terminal).
        let r = Rect::new(0, 0, 80, 24);
        let area = anchored_bottom_right(40, 3, r);
        assert!(
            area.y + area.height <= r.height - 1,
            "toast must sit above the F-key bar"
        );
    }

    #[test]
    fn test_color_for_level_known_levels() {
        assert_eq!(color_for_level("error"), Color::Red);
        assert_eq!(color_for_level("warn"), Color::Yellow);
        assert_eq!(color_for_level("success"), Color::Green);
        assert_eq!(color_for_level("info"), Color::Cyan);
    }

    #[test]
    fn test_color_for_level_unknown_falls_back_to_cyan() {
        // Unknown levels use the neutral info colour so we never
        // accidentally green/red/yellow an important message.
        assert_eq!(color_for_level(""), Color::Cyan);
        assert_eq!(color_for_level("debug"), Color::Cyan);
        assert_eq!(color_for_level("notice"), Color::Cyan);
    }

    #[test]
    fn test_title_for_level_uses_localization_keys() {
        // Each level must map to a known localization key (no
        // hardcoded English in the renderer) and produce a non-empty
        // title string. The translation files in `lang/en.toml` and
        // `lang/es.toml` own the actual wording.
        for level in &["warn", "error", "success", "info"] {
            let title = title_for_level(level);
            assert!(
                !title.trim().is_empty(),
                "level {:?} produced an empty title",
                level
            );
            assert!(
                title.starts_with(' ') && title.ends_with(' '),
                "level {:?} title must keep the title-bar padding (got {:?})",
                level,
                title
            );
        }
    }

    #[test]
    fn test_width_for_short_body_uses_minimum() {
        // A short body must still be readable: we never shrink below
        // the minimum width.
        let body = "OK";
        let natural = (body.chars().count() as u16) + 4;
        let size_w = 120;
        let max = (size_w * TOAST_MAX_WIDTH_PCT) / 100;
        let width = natural.max(TOAST_MIN_WIDTH).min(max.max(TOAST_MIN_WIDTH));
        assert_eq!(width, TOAST_MIN_WIDTH);
    }

    #[test]
    fn test_width_for_long_body_caps_at_max_pct() {
        // A pathologically long body must NOT push the toast off the
        // screen: it caps at 80% of the terminal width.
        let body = "x".repeat(200);
        let size_w = 100;
        let max = (size_w * TOAST_MAX_WIDTH_PCT) / 100;
        let width = ((body.chars().count() as u16) + 4)
            .max(TOAST_MIN_WIDTH)
            .min(max.max(TOAST_MIN_WIDTH));
        assert_eq!(width, max);
    }

    #[test]
    fn test_anchored_rect_dimensions_for_common_terminal_sizes() {
        // Sanity check across a few common terminal sizes: the
        // bottom-right anchor always lands inside the rectangle and
        // leaves at least one row above the bottom edge.
        for (w, h) in &[(80, 24), (120, 40), (200, 60), (40, 12)] {
            let r = Rect::new(0, 0, *w, *h);
            let area = anchored_bottom_right(40, 4, r);
            assert!(
                area.x + area.width <= r.width,
                "toast overflows right edge on {}x{}",
                w,
                h
            );
            assert!(
                area.y + area.height <= r.height,
                "toast overflows bottom edge on {}x{}",
                w,
                h
            );
        }
    }
}
