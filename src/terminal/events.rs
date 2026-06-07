use crossterm::event::{self, Event as CrossEvent, KeyEvent, MouseEvent};
use std::time::Duration;
use tokio::sync::mpsc;

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub enum Event {
    /// Keyboard key pressed
    Key(KeyEvent),
    /// Mouse action occurred
    Mouse(MouseEvent),
    /// Terminal window resized
    Resize(u16, u16),
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
            loop {
                // Poll for new input event with timeout
                match event::poll(tick_rate) {
                    Ok(true) => {
                        #[allow(clippy::collapsible_match)]
                        match event::read() {
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
                },
                Ok(false) => {
                        // Timeout reached, send Tick event
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
