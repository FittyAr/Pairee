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
