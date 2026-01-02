//! Text input component

use crate::event::keys;
use crossterm::event::KeyEvent;

/// A text input field with cursor support
#[derive(Debug, Default, Clone)]
pub struct TextInput {
    /// Current input value
    value: String,
    /// Cursor position
    cursor: usize,
    /// Whether the input is focused
    focused: bool,
}

impl TextInput {
    /// Create a new text input
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the current value
    pub fn value(&self) -> &str {
        &self.value
    }

    /// Set the value
    #[allow(dead_code)]
    pub fn set_value(&mut self, value: String) {
        self.value = value;
        self.cursor = self.value.len();
    }

    /// Clear the input
    pub fn clear(&mut self) {
        self.value.clear();
        self.cursor = 0;
    }

    /// Get cursor position
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// Check if focused
    pub fn is_focused(&self) -> bool {
        self.focused
    }

    /// Set focus state
    pub fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
    }

    /// Handle a key event
    pub fn handle_key(&mut self, key: &KeyEvent) -> bool {
        if !self.focused {
            return false;
        }

        if let Some(c) = keys::get_char(key) {
            // Insert character at cursor
            self.value.insert(self.cursor, c);
            self.cursor += 1;
            return true;
        }

        if keys::is_backspace(key) && self.cursor > 0 {
            self.cursor -= 1;
            self.value.remove(self.cursor);
            return true;
        }

        if keys::is_delete(key) && self.cursor < self.value.len() {
            self.value.remove(self.cursor);
            return true;
        }

        if keys::is_left(key) && self.cursor > 0 {
            self.cursor -= 1;
            return true;
        }

        if keys::is_right(key) && self.cursor < self.value.len() {
            self.cursor += 1;
            return true;
        }

        if keys::is_home(key) {
            self.cursor = 0;
            return true;
        }

        if keys::is_end(key) {
            self.cursor = self.value.len();
            return true;
        }

        // Ctrl+U to clear
        if keys::is_ctrl_char(key, 'u') {
            self.clear();
            return true;
        }

        // Ctrl+W to delete word
        if keys::is_ctrl_char(key, 'w') {
            while self.cursor > 0 && self.value.chars().nth(self.cursor - 1) == Some(' ') {
                self.cursor -= 1;
                self.value.remove(self.cursor);
            }
            while self.cursor > 0 && self.value.chars().nth(self.cursor - 1) != Some(' ') {
                self.cursor -= 1;
                self.value.remove(self.cursor);
            }
            return true;
        }

        false
    }

    /// Get the value before cursor
    #[allow(dead_code)]
    pub fn value_before_cursor(&self) -> &str {
        &self.value[..self.cursor]
    }

    /// Get the value after cursor
    #[allow(dead_code)]
    pub fn value_after_cursor(&self) -> &str {
        &self.value[self.cursor..]
    }

    /// Take the value and clear the input
    pub fn take(&mut self) -> String {
        let value = std::mem::take(&mut self.value);
        self.cursor = 0;
        value
    }
}
