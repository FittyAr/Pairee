use crossterm::event::{self, Event as CrossEvent, KeyEvent, KeyModifiers, MouseEvent};
use std::time::Duration;
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub enum Event {
    /// Keyboard key pressed
    Key(KeyEvent),
    /// Mouse action occurred
    Mouse(MouseEvent),
    /// Terminal window resized
    Resize(u16, u16),
    /// Modifier state changed (poll on tick)
    ModifiersChanged(crossterm::event::KeyModifiers),
    /// Bracketed-paste payload (single string, not per-character key events).
    Paste(String),
    /// Signal-requested application termination (SIGINT, SIGTERM, SIGHUP, SIGQUIT on Unix)
    Terminate,
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

        #[cfg(unix)]
        {
            let sig_sender = sender.clone();
            let _ = crate::terminal::signals::spawn_signal_listener(sig_sender);
        }

        #[cfg(windows)]
        {
            let sig_sender = sender.clone();
            crate::terminal::signals_windows::setup_windows_ctrl_handler(sig_sender);
        }

        std::thread::spawn(move || {
            InputPump {
                sender,
                last_modifiers: KeyModifiers::empty(),
                has_focus: true,
            }
            .run(tick_rate)
        });

        Self { receiver }
    }

    /// Asynchronously receives the next terminal input or tick event.
    pub async fn next(&mut self) -> Option<Event> {
        self.receiver.recv().await
    }
}

/// Input thread state: forwards Crossterm events and, on each tick, the
/// modifier keys held down (where the platform can report them).
struct InputPump {
    sender: mpsc::Sender<Event>,
    last_modifiers: KeyModifiers,
    // Only read by the Windows/Linux modifier polling.
    #[cfg_attr(not(any(windows, target_os = "linux")), allow(dead_code))]
    has_focus: bool,
}

impl InputPump {
    fn run(mut self, tick_rate: Duration) {
        loop {
            // Poll for new input event with timeout
            let keep_going = match event::poll(tick_rate) {
                Ok(true) => match event::read() {
                    Ok(ev) => self.forward(ev),
                    Err(_) => true,
                },
                // Timeout reached, check modifiers then send Tick event
                Ok(false) => self.tick(),
                Err(_) => false,
            };
            if !keep_going {
                break;
            }
        }
    }

    /// Forwards one Crossterm event. Returns `false` once the receiver is gone.
    fn forward(&mut self, ev: CrossEvent) -> bool {
        let event = match ev {
            CrossEvent::FocusGained => {
                self.has_focus = true;
                return true;
            }
            CrossEvent::FocusLost => {
                self.has_focus = false;
                if !self.last_modifiers.is_empty() {
                    self.last_modifiers = KeyModifiers::empty();
                    let _ = self
                        .sender
                        .blocking_send(Event::ModifiersChanged(self.last_modifiers));
                }
                return true;
            }
            CrossEvent::Key(key) => Event::Key(key),
            CrossEvent::Mouse(mouse) => Event::Mouse(mouse),
            CrossEvent::Resize(w, h) => Event::Resize(w, h),
            CrossEvent::Paste(text) => Event::Paste(text),
        };
        self.sender.blocking_send(event).is_ok()
    }

    /// Reports changed modifiers, then a tick. Returns `false` once the receiver is gone.
    fn tick(&mut self) -> bool {
        #[cfg(any(windows, target_os = "linux"))]
        if self.has_focus
            && let Some(current_modifiers) = held_modifiers()
            && current_modifiers != self.last_modifiers
        {
            self.last_modifiers = current_modifiers;
            let _ = self
                .sender
                .blocking_send(Event::ModifiersChanged(current_modifiers));
        }
        self.sender.blocking_send(Event::Tick).is_ok()
    }
}

/// Modifier keys currently held down (Windows: `GetAsyncKeyState`).
#[cfg(windows)]
fn held_modifiers() -> Option<KeyModifiers> {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        GetAsyncKeyState, VK_CONTROL, VK_MENU, VK_SHIFT,
    };
    let keys = [
        (VK_CONTROL, KeyModifiers::CONTROL),
        (VK_MENU, KeyModifiers::ALT),
        (VK_SHIFT, KeyModifiers::SHIFT),
    ];
    let mut current_modifiers = KeyModifiers::empty();
    for (vk, modifier) in keys {
        // SAFETY: GetAsyncKeyState only reads the keyboard state.
        if (unsafe { GetAsyncKeyState(vk as i32) } as u16 & 0x8000) != 0 {
            current_modifiers |= modifier;
        }
    }
    Some(current_modifiers)
}

/// Modifier keys currently held down (Linux: X11 query, when available).
#[cfg(target_os = "linux")]
fn held_modifiers() -> Option<KeyModifiers> {
    super::x11_poll::get_x11_modifiers()
}
