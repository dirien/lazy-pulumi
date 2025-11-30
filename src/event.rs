//! Event handling for the TUI
//!
//! Manages keyboard input, terminal events, and tick events
//! using an async channel-based architecture.

use color_eyre::Result;
use crossterm::event::{self, Event as CrosstermEvent, KeyEvent, MouseEvent};
use std::time::Duration;
use tokio::sync::mpsc;

/// Events that can occur in the application
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum Event {
    /// Terminal tick (for animations and updates)
    Tick,
    /// Key press event
    Key(KeyEvent),
    /// Mouse event
    Mouse(MouseEvent),
    /// Terminal resize
    Resize(u16, u16),
    /// Error occurred
    Error(String),
}

/// Event handler that manages terminal events
pub struct EventHandler {
    /// Event receiver
    rx: mpsc::UnboundedReceiver<Event>,
    /// Stop signal sender
    _stop_tx: mpsc::Sender<()>,
}

impl EventHandler {
    /// Create a new event handler with the specified tick rate
    pub fn new(tick_rate: Duration) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        let (stop_tx, mut stop_rx) = mpsc::channel::<()>(1);

        let event_tx = tx.clone();
        tokio::spawn(async move {
            loop {
                // Check for stop signal
                if stop_rx.try_recv().is_ok() {
                    break;
                }

                // Poll for events with timeout
                if event::poll(tick_rate).unwrap_or(false) {
                    match event::read() {
                        Ok(CrosstermEvent::Key(key)) => {
                            if event_tx.send(Event::Key(key)).is_err() {
                                break;
                            }
                        }
                        Ok(CrosstermEvent::Mouse(mouse)) => {
                            if event_tx.send(Event::Mouse(mouse)).is_err() {
                                break;
                            }
                        }
                        Ok(CrosstermEvent::Resize(w, h)) => {
                            if event_tx.send(Event::Resize(w, h)).is_err() {
                                break;
                            }
                        }
                        Ok(_) => {}
                        Err(e) => {
                            let _ = event_tx.send(Event::Error(e.to_string()));
                            break;
                        }
                    }
                } else {
                    // Send tick event
                    if event_tx.send(Event::Tick).is_err() {
                        break;
                    }
                }
            }
        });

        Self {
            rx,
            _stop_tx: stop_tx,
        }
    }

    /// Get the next event
    pub async fn next(&mut self) -> Result<Event> {
        self.rx
            .recv()
            .await
            .ok_or_else(|| color_eyre::eyre::eyre!("Event channel closed"))
    }
}

/// Key bindings configuration
pub mod keys {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    /// Check if key is quit (q or Ctrl+C)
    pub fn is_quit(key: &KeyEvent) -> bool {
        matches!(
            key,
            KeyEvent {
                code: KeyCode::Char('q'),
                modifiers: KeyModifiers::NONE,
                ..
            } | KeyEvent {
                code: KeyCode::Char('c'),
                modifiers: KeyModifiers::CONTROL,
                ..
            }
        )
    }

    /// Check if key is escape
    pub fn is_escape(key: &KeyEvent) -> bool {
        matches!(
            key,
            KeyEvent {
                code: KeyCode::Esc,
                ..
            }
        )
    }

    /// Check if key is enter
    pub fn is_enter(key: &KeyEvent) -> bool {
        matches!(
            key,
            KeyEvent {
                code: KeyCode::Enter,
                ..
            }
        )
    }

    /// Check if key is tab
    pub fn is_tab(key: &KeyEvent) -> bool {
        matches!(
            key,
            KeyEvent {
                code: KeyCode::Tab,
                modifiers: KeyModifiers::NONE,
                ..
            }
        )
    }

    /// Check if key is shift+tab (backtab)
    pub fn is_backtab(key: &KeyEvent) -> bool {
        matches!(
            key,
            KeyEvent {
                code: KeyCode::BackTab,
                ..
            } | KeyEvent {
                code: KeyCode::Tab,
                modifiers: KeyModifiers::SHIFT,
                ..
            }
        )
    }

    /// Check if key is up arrow or k
    pub fn is_up(key: &KeyEvent) -> bool {
        matches!(
            key,
            KeyEvent {
                code: KeyCode::Up | KeyCode::Char('k'),
                modifiers: KeyModifiers::NONE,
                ..
            }
        )
    }

    /// Check if key is down arrow or j
    pub fn is_down(key: &KeyEvent) -> bool {
        matches!(
            key,
            KeyEvent {
                code: KeyCode::Down | KeyCode::Char('j'),
                modifiers: KeyModifiers::NONE,
                ..
            }
        )
    }

    /// Check if key is left arrow or h
    pub fn is_left(key: &KeyEvent) -> bool {
        matches!(
            key,
            KeyEvent {
                code: KeyCode::Left | KeyCode::Char('h'),
                modifiers: KeyModifiers::NONE,
                ..
            }
        )
    }

    /// Check if key is right arrow or l
    pub fn is_right(key: &KeyEvent) -> bool {
        matches!(
            key,
            KeyEvent {
                code: KeyCode::Right | KeyCode::Char('l'),
                modifiers: KeyModifiers::NONE,
                ..
            }
        )
    }

    /// Check if key is page up
    pub fn is_page_up(key: &KeyEvent) -> bool {
        matches!(
            key,
            KeyEvent {
                code: KeyCode::PageUp,
                ..
            }
        )
    }

    /// Check if key is page down
    pub fn is_page_down(key: &KeyEvent) -> bool {
        matches!(
            key,
            KeyEvent {
                code: KeyCode::PageDown,
                ..
            }
        )
    }

    /// Check if key is home
    pub fn is_home(key: &KeyEvent) -> bool {
        matches!(
            key,
            KeyEvent {
                code: KeyCode::Home,
                ..
            }
        )
    }

    /// Check if key is end
    pub fn is_end(key: &KeyEvent) -> bool {
        matches!(
            key,
            KeyEvent {
                code: KeyCode::End,
                ..
            }
        )
    }

    /// Check if key is backspace
    pub fn is_backspace(key: &KeyEvent) -> bool {
        matches!(
            key,
            KeyEvent {
                code: KeyCode::Backspace,
                ..
            }
        )
    }

    /// Check if key is delete
    pub fn is_delete(key: &KeyEvent) -> bool {
        matches!(
            key,
            KeyEvent {
                code: KeyCode::Delete,
                ..
            }
        )
    }

    /// Check for specific character (handles both with and without shift for case-sensitive matching)
    pub fn is_char(key: &KeyEvent, c: char) -> bool {
        matches!(
            key,
            KeyEvent {
                code: KeyCode::Char(ch),
                modifiers: KeyModifiers::NONE | KeyModifiers::SHIFT,
                ..
            } if *ch == c
        )
    }

    /// Check for character with ctrl modifier
    pub fn is_ctrl_char(key: &KeyEvent, c: char) -> bool {
        matches!(
            key,
            KeyEvent {
                code: KeyCode::Char(ch),
                modifiers: KeyModifiers::CONTROL,
                ..
            } if *ch == c
        )
    }

    /// Get the character if it's a printable character
    pub fn get_char(key: &KeyEvent) -> Option<char> {
        if let KeyEvent {
            code: KeyCode::Char(c),
            modifiers: KeyModifiers::NONE | KeyModifiers::SHIFT,
            ..
        } = key
        {
            Some(*c)
        } else {
            None
        }
    }
}
