//! Loading spinner component

use crate::theme::symbols::SPINNER;

/// An animated loading spinner
#[derive(Debug, Clone)]
pub struct Spinner {
    /// Current frame index
    frame: usize,
    /// Message to display
    message: String,
}

impl Default for Spinner {
    fn default() -> Self {
        Self {
            frame: 0,
            message: "Loading...".to_string(),
        }
    }
}

impl Spinner {
    /// Create a new spinner
    pub fn new() -> Self {
        Self::default()
    }

    /// Create with a custom message
    #[allow(dead_code)]
    pub fn with_message(message: impl Into<String>) -> Self {
        Self {
            frame: 0,
            message: message.into(),
        }
    }

    /// Set the message
    pub fn set_message(&mut self, message: impl Into<String>) {
        self.message = message.into();
    }

    /// Get the message
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Advance to the next frame
    pub fn tick(&mut self) {
        self.frame = (self.frame + 1) % SPINNER.len();
    }

    /// Get the current spinner character
    pub fn char(&self) -> &'static str {
        SPINNER[self.frame]
    }

    /// Get the full display string
    #[allow(dead_code)]
    pub fn display(&self) -> String {
        format!("{} {}", self.char(), self.message)
    }
}
