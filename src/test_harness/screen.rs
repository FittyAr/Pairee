//! Rendering the harness state on a `TestBackend` and screen assertions.

use super::Harness;
use crate::terminal::Event;
use ratatui::layout::Rect;

impl Harness {
    /// Paints a frame and returns it as text, one line per row (wide
    /// characters keep their trailing blank cell).
    pub fn screen_text(&self) -> String {
        // `screen` is only a canvas; painting through a shared reference
        // keeps `screen_text` usable inside panic messages.
        let size = self.screen.backend().buffer().area;
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(size.width, size.height))
                .expect("test terminal");
        crate::app::app::paint(&mut terminal, &self.ctx, &self.state).expect("paint");
        let buffer = terminal.backend().buffer();
        let width = usize::from(buffer.area.width.max(1));
        buffer
            .content()
            .chunks(width)
            .map(|row| row.iter().map(|cell| cell.symbol()).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Paints a frame on the harness terminal (records click targets such
    /// as tab bar cells, like the event loop does).
    pub fn render(&mut self) -> &mut Self {
        crate::app::app::paint(&mut self.screen, &self.ctx, &self.state).expect("paint");
        self
    }

    /// Resizes the terminal (with the resize event) and paints a frame.
    pub fn resize(&mut self, width: u16, height: u16) -> &mut Self {
        self.screen
            .resize(Rect::new(0, 0, width, height))
            .expect("resize");
        self.send(Event::Resize(width, height));
        self.render()
    }

    /// Asserts the screen shows `needle`.
    #[track_caller]
    pub fn assert_screen(&self, needle: &str) {
        let text = self.screen_text();
        assert!(text.contains(needle), "{needle:?} not on screen:\n{text}");
    }

    /// Asserts the screen does not show `needle`.
    #[track_caller]
    pub fn assert_no_screen(&self, needle: &str) {
        let text = self.screen_text();
        assert!(
            !text.contains(needle),
            "{needle:?} unexpectedly on screen:\n{text}"
        );
    }

    /// Screen cell (column, row) where `needle` first appears, scanning rows
    /// top down and only from column `from_column` on (to pick a panel).
    #[track_caller]
    pub fn locate_from(&self, needle: &str, from_column: u16) -> (u16, u16) {
        let text = self.screen_text();
        for (row, line) in text.lines().enumerate() {
            let cells: Vec<char> = line.chars().collect();
            let tail: String = cells.iter().skip(usize::from(from_column)).collect();
            if let Some(byte) = tail.find(needle) {
                let column = from_column as usize + tail[..byte].chars().count();
                return (column as u16, row as u16);
            }
        }
        panic!("{needle:?} not on screen from column {from_column}:\n{text}");
    }

    /// Top-left cell of the first area the last frame recorded for a
    /// target `pick` accepts (paints a frame first).
    #[track_caller]
    pub fn hit_cell(&mut self, pick: impl Fn(&crate::ui::hit::HitTarget) -> bool) -> (u16, u16) {
        self.render();
        let (area, ()) = self
            .state
            .hits
            .first(|target| pick(target).then_some(()))
            .unwrap_or_else(|| panic!("no such target on screen:\n{}", self.screen_text()));
        (area.x, area.y)
    }

    /// Screen cell where `needle` first appears.
    #[track_caller]
    pub fn locate(&self, needle: &str) -> (u16, u16) {
        self.locate_from(needle, 0)
    }

    /// Name of the entry under the cursor of the focused panel.
    pub fn cursor_name(&self) -> String {
        let panel = self.state.get_active_panel();
        panel
            .entries
            .get(panel.cursor_index)
            .map(|e| e.name.clone())
            .unwrap_or_default()
    }

    /// Names listed in the focused panel.
    pub fn names(&self) -> Vec<String> {
        let panel = self.state.get_active_panel();
        panel.entries.iter().map(|e| e.name.clone()).collect()
    }

    /// Moves the cursor of the focused panel onto `name` with the arrow
    /// keys (Home, then Down until it is reached).
    pub fn focus(&mut self, name: &str) -> &mut Self {
        self.keys("Home");
        for _ in 0..self.names().len() {
            if self.cursor_name() == name {
                return self;
            }
            self.keys("Down");
        }
        panic!("{name:?} not listed in {:?}", self.names());
    }

    /// Rereads both panels (Ctrl+R equivalent, without a keymap lookup).
    pub fn reread(&mut self) -> &mut Self {
        let show_hidden = self.ctx.config.settings.show_hidden;
        self.state.force_refresh_both_panels(show_hidden);
        self.settle();
        self
    }
}
