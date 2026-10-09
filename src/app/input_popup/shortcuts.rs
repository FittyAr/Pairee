//! Keys of the keyboard-shortcuts modal.
//!
//! Browsing: type to filter, ↑↓ PgUp PgDn Home End move, Tab switches the
//! context, Ctrl+←/→ previews another preset, Enter runs the action, F2
//! rebinds, Ins adds a key, Del removes the keys, F8 restores (Shift+F8:
//! all), F3 finds what a key does, F5 uses the previewed preset, F9 saves
//! the result as a preset file, Esc closes. While a key is being captured,
//! Enter confirms and Esc cancels, so those two cannot be captured.

use crate::app::context::AppContext;
use crate::app::form::{FieldKey, field_key};
use crate::app::list_nav::NavStep;
use crate::app::shortcuts::edit;
use crate::app::shortcuts::state::{Mode, ShortcutsState};
use crate::app::state::{AppState, PopupType};
use crate::config::localization::t;
use crate::keybindings::Action;
use crate::keybindings::chord::fragility;
use crate::keybindings::resolver::key_event_to_string;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    let Some(PopupType::Shortcuts(s)) = state.dialogs.top_mut() else {
        return Err(());
    };
    let outcome = match s.mode.clone() {
        Mode::Browse => browse(s, key, context),
        Mode::Capture { add, keys } => {
            capture(s, key, context, add, keys);
            Outcome::Stay
        }
        Mode::ConfirmReplace { add, chord, .. } => {
            if key.code == KeyCode::Enter {
                commit(s, context, add, chord);
            }
            s.mode = Mode::Browse;
            Outcome::Stay
        }
        Mode::KeySearch => {
            if key.code != KeyCode::Esc && !is_modifier(&key) {
                s.search_key(key_event_to_string(key));
            }
            s.mode = Mode::Browse;
            Outcome::Stay
        }
        Mode::Export { name } => export(s, key, context, name),
        Mode::ConfirmResetAll => {
            if matches!(key.code, KeyCode::Enter | KeyCode::Char('y' | 'Y')) {
                edit::restore_all(context, s);
            }
            s.mode = Mode::Browse;
            Outcome::Stay
        }
    };
    match outcome {
        Outcome::Stay => Ok(None),
        // Esc goes back to the dialog underneath (Settings); running an
        // action closes every dialog, as from the panels.
        Outcome::Close(None) => {
            state.dialogs.pop();
            Ok(None)
        }
        Outcome::Close(action) => {
            state.dialogs.clear();
            Ok(action)
        }
    }
}

enum Outcome {
    Stay,
    Close(Option<Action>),
}

fn is_modifier(key: &KeyEvent) -> bool {
    matches!(key.code, KeyCode::Modifier(_))
}

fn browse(s: &mut ShortcutsState, key: KeyEvent, context: &mut AppContext) -> Outcome {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    let shift = key.modifiers.contains(KeyModifiers::SHIFT);
    s.message = None;
    match key.code {
        KeyCode::Esc if s.key_filter.is_some() => s.key_filter = None,
        KeyCode::Esc => return Outcome::Close(None),
        KeyCode::Enter => return run(s, context),
        KeyCode::Tab | KeyCode::BackTab => {
            s.tab = s.tab.step(if key.code == KeyCode::Tab { 1 } else { -1 });
            s.cursor = 0;
        }
        KeyCode::Left if ctrl => edit::preview(context, s, -1),
        KeyCode::Right if ctrl => edit::preview(context, s, 1),
        KeyCode::F(2) => s.mode = capture_mode(false),
        KeyCode::Insert => s.mode = capture_mode(true),
        KeyCode::Delete => edit::assign(context, s, &[]),
        KeyCode::F(8) if shift => s.mode = Mode::ConfirmResetAll,
        KeyCode::F(8) => edit::restore(context, s),
        KeyCode::F(3) => s.mode = Mode::KeySearch,
        KeyCode::F(5) => edit::activate(context, s),
        KeyCode::F(9) => {
            let name = format!("{}-mine", s.preset).into();
            s.mode = Mode::Export { name };
        }
        code => match NavStep::from_key(code) {
            Some(step) => s.cursor = step.apply_clamped(s.cursor, s.visible().len()),
            None => {
                if field_key(&mut s.query, &key) == FieldKey::Handled {
                    s.key_filter = None;
                    s.cursor = 0;
                }
            }
        },
    }
    Outcome::Stay
}

fn capture_mode(add: bool) -> Mode {
    Mode::Capture {
        add,
        keys: Vec::new(),
    }
}

/// Enter runs the highlighted panel action of the active preset.
fn run(s: &ShortcutsState, context: &AppContext) -> Outcome {
    let active =
        crate::keybindings::embedded::normalize_preset_name(&context.config.keybindings.preset);
    match s.current().and_then(|r| r.run) {
        Some(action) if s.preset == active => Outcome::Close(Some(action)),
        _ => Outcome::Stay,
    }
}

fn capture(
    s: &mut ShortcutsState,
    key: KeyEvent,
    context: &mut AppContext,
    add: bool,
    mut keys: Vec<String>,
) {
    match key.code {
        KeyCode::Esc => s.mode = Mode::Browse,
        KeyCode::Enter if keys.is_empty() => s.mode = Mode::Browse,
        KeyCode::Enter => {
            let chord = keys.join(" ");
            let own = s.current().map(|r| r.id.clone()).unwrap_or_default();
            s.mode = match s.owner_of(&chord, &own) {
                Some(owner) => Mode::ConfirmReplace {
                    add,
                    chord,
                    owner: owner.label.clone(),
                },
                None => {
                    commit(s, context, add, chord);
                    Mode::Browse
                }
            };
        }
        _ if is_modifier(&key) => {}
        _ => {
            keys.push(key_event_to_string(key));
            s.mode = Mode::Capture { add, keys };
        }
    }
}

/// Gives the cursor row `chord` (added to its keys or replacing them).
fn commit(s: &mut ShortcutsState, context: &mut AppContext, add: bool, chord: String) {
    let mut chords = match (add, s.current()) {
        (true, Some(row)) => row.chords.clone(),
        _ => Vec::new(),
    };
    chords.push(chord.clone());
    edit::assign(context, s, &chords);
    if let Some(reason) = chord.parse().ok().as_ref().and_then(fragility) {
        s.message = Some(format!("≈ {chord}: {}", t(reason.label_key())));
    }
}

fn export(
    s: &mut ShortcutsState,
    key: KeyEvent,
    context: &AppContext,
    mut name: crate::app::text_input::TextField,
) -> Outcome {
    match field_key(&mut name, &key) {
        FieldKey::Submit => {
            s.message = Some(match edit::export(context, s, name.text().trim()) {
                Ok(path) => t("shortcuts_exported").replace("{}", &path.display().to_string()),
                Err(e) => e,
            });
            s.mode = Mode::Browse;
        }
        FieldKey::Cancel => s.mode = Mode::Browse,
        FieldKey::Handled | FieldKey::Other => s.mode = Mode::Export { name },
    }
    Outcome::Stay
}
