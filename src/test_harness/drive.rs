//! Feeding input to the harness and waiting for background work.

use super::Harness;
use super::keys::{self, Token};
use crate::app::app::{background, events};
use crate::keybindings::Action;
use crate::terminal::Event;
use crossterm::event::{KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use std::time::{Duration, Instant};

/// How long [`Harness::wait_until`] waits before failing.
const WAIT_TIMEOUT: Duration = Duration::from_secs(20);
/// Pause between two drains of the background channels.
const POLL_INTERVAL: Duration = Duration::from_millis(5);

impl Harness {
    /// Sends a key script (see `keys::parse`), settling after each key.
    pub fn keys(&mut self, script: &str) -> &mut Self {
        for token in keys::parse(script) {
            match token {
                Token::Key(key) => self.send(Event::Key(key)),
                Token::Action(name) => self.trigger(&name),
            }
        }
        self
    }

    /// Types `text` one character at a time, as a terminal would.
    pub fn text(&mut self, text: &str) -> &mut Self {
        for c in text.chars() {
            self.send(Event::Key(keys::typed(c)));
        }
        self
    }

    /// Delivers a bracketed paste.
    pub fn paste(&mut self, text: &str) -> &mut Self {
        self.send(Event::Paste(text.to_string()));
        self
    }

    /// Left click at a cell of the frame.
    pub fn click(&mut self, column: u16, row: u16) -> &mut Self {
        for kind in [
            MouseEventKind::Down(MouseButton::Left),
            MouseEventKind::Up(MouseButton::Left),
        ] {
            self.send(Event::Mouse(MouseEvent {
                kind,
                column,
                row,
                modifiers: KeyModifiers::NONE,
            }));
        }
        self
    }

    /// Presses the chord the active keymap binds to action `name`; an
    /// unbound action (e.g. `mkdir` in the Norton keymap, reached from the
    /// menu there) is dispatched directly.
    pub fn trigger(&mut self, name: &str) {
        let action = crate::keybindings::preset::parse_action_name(name)
            .unwrap_or_else(|| panic!("unknown action {name:?}"));
        match self.chord_for(action) {
            Some(key) => self.send(Event::Key(key)),
            None => self.dispatch(action),
        }
    }

    /// The first chord bound to `action` in the active keymap.
    pub fn chord_for(&self, action: Action) -> Option<KeyEvent> {
        let chord = self.ctx.resolver.key_for_action(action)?;
        let first = chord.split(',').next().unwrap_or(chord).trim();
        Some(
            keys::parse_chord(first)
                .unwrap_or_else(|| panic!("cannot replay chord {chord:?} of {action:?}")),
        )
    }

    /// Runs `action` as the keymap would, without a key press.
    pub fn dispatch(&mut self, action: Action) {
        let Self {
            state,
            ctx,
            rt,
            backend,
            ..
        } = self;
        rt.block_on(crate::app::actions::handle_action(
            state, action, ctx, backend,
        ))
        .expect("action");
        self.settle();
    }

    /// Delivers one input event through the event-loop dispatcher.
    pub fn send(&mut self, event: Event) {
        let Self {
            state,
            ctx,
            rt,
            backend,
            ..
        } = self;
        state.mark_ui_dirty();
        rt.block_on(events::handle_input_event(state, ctx, backend, event))
            .expect("input event");
        self.settle();
    }

    /// Drains the background channels once (one event-loop tick).
    pub fn pump(&mut self) {
        let _guard = self.rt.enter();
        background::process_background_updates(&mut self.state, &self.ctx, &mut self.backend);
    }

    /// Pumps until no tracked background job is running, then once more so
    /// results that were just delivered are applied.
    pub fn settle(&mut self) {
        let started = Instant::now();
        loop {
            self.pump();
            if !self.busy() {
                self.pump();
                return;
            }
            if started.elapsed() > WAIT_TIMEOUT {
                panic!("background work did not finish\n{}", self.screen_text());
            }
            std::thread::sleep(POLL_INTERVAL);
        }
    }

    /// Pumps until `done` holds; fails with the screen after a timeout.
    pub fn wait_until(&mut self, what: &str, mut done: impl FnMut(&mut Self) -> bool) {
        let started = Instant::now();
        loop {
            self.pump();
            if done(self) {
                return;
            }
            if started.elapsed() > WAIT_TIMEOUT {
                panic!("timed out waiting for {what}\n{}", self.screen_text());
            }
            std::thread::sleep(POLL_INTERVAL);
        }
    }

    /// Waits until the rendered screen shows `needle`.
    pub fn wait_for_text(&mut self, needle: &str) {
        self.wait_until(&format!("{needle:?} on screen"), |h| {
            h.screen_text().contains(needle)
        });
    }

    /// True while a background job the harness knows about is running.
    fn busy(&self) -> bool {
        let state = &self.state;
        let listing = [
            crate::app::state::ActivePanel::Left,
            crate::app::state::ActivePanel::Right,
        ]
        .into_iter()
        .flat_map(|side| state.panels.tabs(side).tabs())
        .any(|tab| tab.panel.listing.is_running() || tab.panel.dir_sizes.is_running());
        // A job waiting for a conflict answer is idle until a key arrives.
        let transfer = state.transfer.as_ref().is_some_and(|t| {
            t.active_conflict_info.is_none()
                && t.engine
                    .queue
                    .get_all()
                    .iter()
                    .any(|job| !job.is_terminal())
        });
        listing
            || transfer
            || state.git_op.is_running()
            || state.multi_rename.is_running()
            || state.vfs_op.is_running()
    }
}
