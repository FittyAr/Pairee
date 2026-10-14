//! The screens (panels, viewers, editors, terminals) the user switches
//! between. Each screen has a stable [`ScreenId`], so background output
//! still reaches its screen after others were closed, and keeps the dialogs
//! that were open on it while another screen is shown.

use super::types::Screen;
use super::{AppState, DialogStack};

/// Identity of a screen for as long as it is open (never reused).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ScreenId(u64);

#[derive(Debug)]
struct Entry {
    id: ScreenId,
    screen: Screen,
    /// Dialogs of this screen while it is not the active one.
    parked: DialogStack,
}

/// Open screens; the first one is always the panels.
#[derive(Debug)]
pub struct ScreenStack {
    entries: Vec<Entry>,
    active: usize,
    next_id: u64,
    /// Output lines a new terminal screen keeps (`terminal_scrollback_lines`).
    pub terminal_lines: usize,
}

impl Default for ScreenStack {
    fn default() -> Self {
        Self {
            entries: vec![Entry {
                id: ScreenId(0),
                screen: Screen::Panels,
                parked: DialogStack::new(),
            }],
            active: 0,
            next_id: 1,
            terminal_lines: super::terminal::DEFAULT_LINES,
        }
    }
}

impl ScreenStack {
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Always false: the panels screen cannot be closed.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Position of the active screen (0 = panels).
    pub fn active_index(&self) -> usize {
        self.active
    }

    pub fn active_id(&self) -> ScreenId {
        self.entries[self.active].id
    }

    pub fn active(&self) -> &Screen {
        &self.entries[self.active].screen
    }

    pub fn active_mut(&mut self) -> &mut Screen {
        &mut self.entries[self.active].screen
    }

    pub fn get(&self, index: usize) -> Option<&Screen> {
        self.entries.get(index).map(|e| &e.screen)
    }

    /// Id of the screen at `index`.
    pub fn id_at(&self, index: usize) -> Option<ScreenId> {
        self.entries.get(index).map(|e| e.id)
    }

    pub fn index_of(&self, id: ScreenId) -> Option<usize> {
        self.entries.iter().position(|e| e.id == id)
    }

    pub fn by_id_mut(&mut self, id: ScreenId) -> Option<&mut Screen> {
        self.entries
            .iter_mut()
            .find(|e| e.id == id)
            .map(|e| &mut e.screen)
    }

    pub fn last(&self) -> Option<&Screen> {
        self.entries.last().map(|e| &e.screen)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Screen> {
        self.entries.iter().map(|e| &e.screen)
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Screen> {
        self.entries.iter_mut().map(|e| &mut e.screen)
    }

    /// Applies changed highlighting settings (or reloaded definitions) to
    /// the open editors and viewers; a viewer still loading applies them
    /// once its content arrives.
    pub fn apply_syntax_limits(&mut self, limits: &crate::syntax::SyntaxLimits) {
        for screen in self.iter_mut() {
            match screen {
                Screen::Editor(ed) => ed.detect_syntax(limits),
                Screen::Viewer(vw) if vw.loading => vw.pending_syntax = Some(*limits),
                Screen::Viewer(vw) => vw.detect_syntax(limits),
                _ => {}
            }
        }
    }

    /// Makes screen `index` active: `dialogs` (the active screen's) are
    /// parked and replaced by the ones parked on `index`.
    fn activate(&mut self, index: usize, dialogs: &mut DialogStack) {
        if index == self.active || index >= self.entries.len() {
            return;
        }
        self.entries[self.active].parked = std::mem::take(dialogs);
        self.active = index;
        *dialogs = std::mem::take(&mut self.entries[index].parked);
    }

    fn push(&mut self, screen: Screen, dialogs: &mut DialogStack) -> ScreenId {
        let id = ScreenId(self.next_id);
        self.next_id += 1;
        self.entries.push(Entry {
            id,
            screen,
            parked: DialogStack::new(),
        });
        self.activate(self.entries.len() - 1, dialogs);
        id
    }

    /// Removes screen `id` (never the panels). When it was active, the one
    /// before it becomes active. Returns the removed screen.
    fn remove(&mut self, id: ScreenId, dialogs: &mut DialogStack) -> Option<Screen> {
        let index = self.index_of(id).filter(|&i| i > 0)?;
        if index == self.active {
            self.activate(index - 1, dialogs);
            // The closed screen's own dialogs were parked on it: drop them.
        } else if index < self.active {
            self.active -= 1;
        }
        Some(self.entries.remove(index).screen)
    }
}

impl AppState {
    /// Adds a new screen to the stack and makes it active.
    pub fn push_screen(&mut self, screen: Screen) -> ScreenId {
        self.screens.push(screen, &mut self.dialogs)
    }

    /// Shows screen `index` (Screens list, F12).
    pub fn switch_screen(&mut self, index: usize) {
        self.screens.activate(index, &mut self.dialogs);
    }

    /// Switches to the next screen (Ctrl-Tab).
    pub fn next_screen(&mut self) {
        let next = (self.screens.active + 1) % self.screens.len();
        self.switch_screen(next);
    }

    /// Switches to the previous screen (Ctrl-Shift-Tab).
    pub fn prev_screen(&mut self) {
        let len = self.screens.len();
        self.switch_screen((self.screens.active + len - 1) % len);
    }

    /// Closes screen `id` (a terminal screen's process stops when its
    /// state is dropped). The panels screen cannot be closed.
    pub fn close_screen(&mut self, id: ScreenId) {
        self.screens.remove(id, &mut self.dialogs);
    }

    /// Closes the currently active screen, reverting to the previous one.
    pub fn close_current_screen(&mut self) {
        self.close_screen(self.screens.active_id());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::state::PopupType;
    use std::path::PathBuf;

    fn viewer(name: &str) -> Screen {
        Screen::Viewer(crate::ui::viewer::ViewerState::from_text(
            PathBuf::from(name),
            Vec::new(),
        ))
    }

    #[test]
    fn ids_stay_valid_when_earlier_screens_close() {
        let mut state = AppState::new(".".into(), ".".into());
        let a = state.push_screen(viewer("a"));
        let b = state.push_screen(viewer("b"));
        assert_eq!(state.screens.active_index(), 2);
        state.close_screen(a);
        assert_eq!(state.screens.len(), 2);
        assert_eq!(state.screens.active_id(), b, "b stays active");
        assert_eq!(state.screens.index_of(b), Some(1));
        assert!(state.screens.by_id_mut(a).is_none());
        state.close_screen(state.screens.id_at(0).unwrap());
        assert_eq!(state.screens.len(), 2, "the panels never close");
    }

    #[test]
    fn dialogs_are_parked_per_screen() {
        let mut state = AppState::new(".".into(), ".".into());
        state.dialogs.replace(PopupType::info("panels"));
        let v = state.push_screen(viewer("v"));
        assert!(state.dialogs.is_empty());
        state.prev_screen();
        assert!(matches!(state.dialogs.top(), Some(PopupType::Info(_))));
        state.next_screen();
        assert_eq!(state.screens.active_id(), v);
        state.close_current_screen();
        assert_eq!(state.screens.active_index(), 0);
        assert!(matches!(state.dialogs.top(), Some(PopupType::Info(_))));
    }

    #[test]
    fn highlighting_settings_reach_open_screens() {
        use crate::syntax::SyntaxLimits;
        let mut state = AppState::new(".".into(), ".".into());
        state.push_screen(viewer("a.rs"));
        let highlighted = |state: &AppState| match state.screens.last() {
            Some(Screen::Viewer(vw)) => vw.syntax.borrow().is_some(),
            _ => unreachable!(),
        };
        assert!(!highlighted(&state), "opened with highlighting off");
        let on = SyntaxLimits {
            enabled: true,
            max_bytes: 1024,
        };
        state.screens.apply_syntax_limits(&on);
        assert!(highlighted(&state));
        state.screens.apply_syntax_limits(&SyntaxLimits::default());
        assert!(!highlighted(&state));
    }
}
