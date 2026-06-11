
use crossterm::event::{self, Event as CrossEvent, KeyEvent, MouseEvent};
use std::time::Duration;
use tokio::sync::mpsc;

#[derive(Debug, Clone, Copy)]
pub enum Event {
    /// Keyboard key pressed
    Key(KeyEvent),
    /// Mouse action occurred
    Mouse(MouseEvent),
    /// Terminal window resized
    Resize(u16, u16),
    /// Modifier state changed (poll on tick)
    ModifiersChanged(crossterm::event::KeyModifiers),
    /// Periodic tick event for UI updates
    Tick,
}

pub struct EventHandler {
    receiver: mpsc::Receiver<Event>,
}

impl EventHandler {
    /// Starts a background thread polling Crossterm input events and returns the handler.
    pub fn new(tick_rate: Duration) -> Self {
        let (sender, receiver) = mpsc::channel(100);

        std::thread::spawn(move || {
            let mut last_modifiers = crossterm::event::KeyModifiers::empty();
            let mut has_focus = true;

            loop {
                // Poll for new input event with timeout
                match event::poll(tick_rate) {
                    Ok(true) =>
                    {
                        #[allow(clippy::collapsible_match)]
                        match event::read() {
                            Ok(CrossEvent::FocusGained) => {
                                has_focus = true;
                            }
                            Ok(CrossEvent::FocusLost) => {
                                has_focus = false;
                                if !last_modifiers.is_empty() {
                                    last_modifiers = crossterm::event::KeyModifiers::empty();
                                    let _ = sender
                                        .blocking_send(Event::ModifiersChanged(last_modifiers));
                                }
                            }
                            Ok(CrossEvent::Key(key)) => {
                                if sender.blocking_send(Event::Key(key)).is_err() {
                                    break;
                                }
                            }
                            Ok(CrossEvent::Mouse(mouse)) => {
                                if sender.blocking_send(Event::Mouse(mouse)).is_err() {
                                    break;
                                }
                            }
                            Ok(CrossEvent::Resize(w, h)) => {
                                if sender.blocking_send(Event::Resize(w, h)).is_err() {
                                    break;
                                }
                            }
                            _ => {}
                        }
                    }
                    Ok(false) => {
                        // Timeout reached, check modifiers then send Tick event
                        #[cfg(windows)]
                        {
                            if has_focus {
                                use crossterm::event::KeyModifiers;
                                use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
                                    GetAsyncKeyState, VK_CONTROL, VK_MENU, VK_SHIFT,
                                };

                                unsafe {
                                    let mut current_modifiers = KeyModifiers::empty();

                                    if (GetAsyncKeyState(VK_CONTROL as i32) as u16 & 0x8000) != 0 {
                                        current_modifiers |= KeyModifiers::CONTROL;
                                    }
                                    if (GetAsyncKeyState(VK_MENU as i32) as u16 & 0x8000) != 0 {
                                        current_modifiers |= KeyModifiers::ALT;
                                    }
                                    if (GetAsyncKeyState(VK_SHIFT as i32) as u16 & 0x8000) != 0 {
                                        current_modifiers |= KeyModifiers::SHIFT;
                                    }

                                    if current_modifiers != last_modifiers {
                                        last_modifiers = current_modifiers;
                                        let _ = sender.blocking_send(Event::ModifiersChanged(
                                            current_modifiers,
                                        ));
                                    }
                                }
                            }
                        }

                        #[cfg(target_os = "linux")]
                        {
                            if has_focus {
                                if let Some(current_modifiers) = super::x11_poll::get_x11_modifiers() {
                                    if current_modifiers != last_modifiers {
                                        last_modifiers = current_modifiers;
                                        let _ = sender.blocking_send(Event::ModifiersChanged(
                                            current_modifiers,
                                        ));
                                    }
                                }
                            }
                        }

                        if sender.blocking_send(Event::Tick).is_err() {
                            break;
                        }
                    }
                    Err(_) => {
                        break;
                    }
                }
            }
        });

        Self { receiver }
    }

    /// Asynchronously receives the next terminal input or tick event.
    pub async fn next(&mut self) -> Option<Event> {
        self.receiver.recv().await
    }
}
