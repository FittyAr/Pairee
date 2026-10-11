//! Click counting: crossterm reports presses, not double clicks.

use crossterm::event::MouseButton;
use std::time::{Duration, Instant};

/// Most clicks counted in a row (a triple click); the next press starts over.
const MAX_CLICKS: u8 = 3;

/// Counts presses of the same button on the same cell in quick succession.
#[derive(Debug, Default)]
pub struct ClickCounter {
    last: Option<Press>,
}

#[derive(Debug, Clone, Copy)]
struct Press {
    button: MouseButton,
    column: u16,
    row: u16,
    at: Instant,
    count: u8,
}

impl ClickCounter {
    /// Registers a press and returns how many clicks it completes:
    /// 1 for a single click, 2 for a double click, 3 for a triple click.
    pub fn press(
        &mut self,
        button: MouseButton,
        column: u16,
        row: u16,
        now: Instant,
        interval: Duration,
    ) -> u8 {
        let count = match self.last {
            Some(last)
                if last.button == button
                    && last.column == column
                    && last.row == row
                    && now.saturating_duration_since(last.at) <= interval
                    && last.count < MAX_CLICKS =>
            {
                last.count + 1
            }
            _ => 1,
        };
        self.last = Some(Press {
            button,
            column,
            row,
            at: now,
            count,
        });
        count
    }

    /// Forgets the last press (after a drag, so the next press is a single).
    pub fn reset(&mut self) {
        self.last = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const INTERVAL: Duration = Duration::from_millis(400);

    #[test]
    fn quick_presses_on_one_cell_count_up_to_three() {
        let mut counter = ClickCounter::default();
        let t = Instant::now();
        let ms = Duration::from_millis;
        assert_eq!(counter.press(MouseButton::Left, 5, 5, t, INTERVAL), 1);
        assert_eq!(
            counter.press(MouseButton::Left, 5, 5, t + ms(100), INTERVAL),
            2
        );
        assert_eq!(
            counter.press(MouseButton::Left, 5, 5, t + ms(200), INTERVAL),
            3
        );
        assert_eq!(
            counter.press(MouseButton::Left, 5, 5, t + ms(300), INTERVAL),
            1
        );
    }

    #[test]
    fn slow_moved_or_other_button_presses_start_over() {
        let mut counter = ClickCounter::default();
        let t = Instant::now();
        counter.press(MouseButton::Left, 5, 5, t, INTERVAL);
        let late = t + Duration::from_millis(500);
        assert_eq!(counter.press(MouseButton::Left, 5, 5, late, INTERVAL), 1);
        assert_eq!(counter.press(MouseButton::Left, 6, 5, late, INTERVAL), 1);
        assert_eq!(counter.press(MouseButton::Right, 6, 5, late, INTERVAL), 1);
    }

    #[test]
    fn reset_forgets_the_last_press() {
        let mut counter = ClickCounter::default();
        let t = Instant::now();
        counter.press(MouseButton::Left, 1, 1, t, INTERVAL);
        counter.reset();
        assert_eq!(counter.press(MouseButton::Left, 1, 1, t, INTERVAL), 1);
    }
}
