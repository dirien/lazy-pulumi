//! Multi-line text editor component for YAML editing

use crate::event::keys;
use crossterm::event::KeyEvent;

/// A multi-line text editor with cursor support
#[derive(Debug, Clone)]
pub struct TextEditor {
    /// Lines of text
    lines: Vec<String>,
    /// Cursor row (line index)
    row: usize,
    /// Cursor column (character index within line)
    col: usize,
    /// Scroll offset (first visible line)
    scroll_offset: usize,
    /// Visible height (for scrolling)
    visible_height: usize,
    /// Whether the editor has been modified
    modified: bool,
}

impl Default for TextEditor {
    fn default() -> Self {
        Self {
            lines: vec![String::new()],
            row: 0,
            col: 0,
            scroll_offset: 0,
            visible_height: 20,
            modified: false,
        }
    }
}

impl TextEditor {
    /// Create a new text editor
    pub fn new() -> Self {
        Self::default()
    }

    /// Create editor with initial content
    pub fn with_content(content: &str) -> Self {
        let lines: Vec<String> = if content.is_empty() {
            vec![String::new()]
        } else {
            content.lines().map(|l| l.to_string()).collect()
        };

        // Ensure at least one line
        let lines = if lines.is_empty() {
            vec![String::new()]
        } else {
            lines
        };

        Self {
            lines,
            row: 0,
            col: 0,
            scroll_offset: 0,
            visible_height: 20,
            modified: false,
        }
    }

    /// Set the visible height for scrolling
    #[allow(dead_code)]
    pub fn set_visible_height(&mut self, height: usize) {
        self.visible_height = height.max(1);
        self.ensure_cursor_visible();
    }

    /// Get all content as a single string
    pub fn content(&self) -> String {
        self.lines.join("\n")
    }

    /// Get lines for rendering
    pub fn lines(&self) -> &[String] {
        &self.lines
    }

    /// Get current cursor position (row, col)
    pub fn cursor(&self) -> (usize, usize) {
        (self.row, self.col)
    }

    /// Get scroll offset
    pub fn scroll_offset(&self) -> usize {
        self.scroll_offset
    }

    /// Check if modified
    pub fn is_modified(&self) -> bool {
        self.modified
    }

    /// Get total line count
    #[allow(dead_code)]
    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    /// Get current line
    #[allow(dead_code)]
    pub fn current_line(&self) -> &str {
        &self.lines[self.row]
    }

    /// Ensure cursor is visible (adjust scroll)
    fn ensure_cursor_visible(&mut self) {
        if self.row < self.scroll_offset {
            self.scroll_offset = self.row;
        } else if self.row >= self.scroll_offset + self.visible_height {
            self.scroll_offset = self.row.saturating_sub(self.visible_height - 1);
        }
    }

    /// Clamp column to valid range for current line
    fn clamp_col(&mut self) {
        let line_len = self.lines[self.row].len();
        if self.col > line_len {
            self.col = line_len;
        }
    }

    /// Handle a key event, returns true if handled
    pub fn handle_key(&mut self, key: &KeyEvent) -> bool {
        // Character input
        if let Some(c) = keys::get_char(key) {
            self.insert_char(c);
            return true;
        }

        // Navigation
        if keys::is_up(key) {
            self.move_up();
            return true;
        }
        if keys::is_down(key) {
            self.move_down();
            return true;
        }
        if keys::is_left(key) {
            self.move_left();
            return true;
        }
        if keys::is_right(key) {
            self.move_right();
            return true;
        }
        if keys::is_home(key) {
            self.col = 0;
            return true;
        }
        if keys::is_end(key) {
            self.col = self.lines[self.row].len();
            return true;
        }
        if keys::is_page_up(key) {
            self.page_up();
            return true;
        }
        if keys::is_page_down(key) {
            self.page_down();
            return true;
        }

        // Editing
        if keys::is_enter(key) {
            self.insert_newline();
            return true;
        }
        if keys::is_backspace(key) {
            self.backspace();
            return true;
        }
        if keys::is_delete(key) {
            self.delete();
            return true;
        }
        if keys::is_tab(key) {
            // Insert 2 spaces for YAML indentation
            self.insert_char(' ');
            self.insert_char(' ');
            return true;
        }

        // Ctrl shortcuts
        if keys::is_ctrl_char(key, 'u') {
            // Clear line before cursor
            self.lines[self.row] = self.lines[self.row][self.col..].to_string();
            self.col = 0;
            self.modified = true;
            return true;
        }
        if keys::is_ctrl_char(key, 'k') {
            // Clear line after cursor
            self.lines[self.row].truncate(self.col);
            self.modified = true;
            return true;
        }
        if keys::is_ctrl_char(key, 'a') {
            // Go to beginning of line
            self.col = 0;
            return true;
        }
        if keys::is_ctrl_char(key, 'e') {
            // Go to end of line
            self.col = self.lines[self.row].len();
            return true;
        }
        if keys::is_ctrl_char(key, 'd') {
            // Delete line
            if self.lines.len() > 1 {
                self.lines.remove(self.row);
                if self.row >= self.lines.len() {
                    self.row = self.lines.len() - 1;
                }
                self.clamp_col();
                self.ensure_cursor_visible();
                self.modified = true;
            } else {
                self.lines[0].clear();
                self.col = 0;
                self.modified = true;
            }
            return true;
        }

        false
    }

    fn insert_char(&mut self, c: char) {
        self.lines[self.row].insert(self.col, c);
        self.col += 1;
        self.modified = true;
    }

    fn insert_newline(&mut self) {
        let rest = self.lines[self.row].split_off(self.col);
        self.row += 1;
        self.lines.insert(self.row, rest);
        self.col = 0;
        self.ensure_cursor_visible();
        self.modified = true;
    }

    fn backspace(&mut self) {
        if self.col > 0 {
            self.col -= 1;
            self.lines[self.row].remove(self.col);
            self.modified = true;
        } else if self.row > 0 {
            // Merge with previous line
            let current_line = self.lines.remove(self.row);
            self.row -= 1;
            self.col = self.lines[self.row].len();
            self.lines[self.row].push_str(&current_line);
            self.ensure_cursor_visible();
            self.modified = true;
        }
    }

    fn delete(&mut self) {
        if self.col < self.lines[self.row].len() {
            self.lines[self.row].remove(self.col);
            self.modified = true;
        } else if self.row + 1 < self.lines.len() {
            // Merge with next line
            let next_line = self.lines.remove(self.row + 1);
            self.lines[self.row].push_str(&next_line);
            self.modified = true;
        }
    }

    fn move_up(&mut self) {
        if self.row > 0 {
            self.row -= 1;
            self.clamp_col();
            self.ensure_cursor_visible();
        }
    }

    fn move_down(&mut self) {
        if self.row + 1 < self.lines.len() {
            self.row += 1;
            self.clamp_col();
            self.ensure_cursor_visible();
        }
    }

    fn move_left(&mut self) {
        if self.col > 0 {
            self.col -= 1;
        } else if self.row > 0 {
            self.row -= 1;
            self.col = self.lines[self.row].len();
            self.ensure_cursor_visible();
        }
    }

    fn move_right(&mut self) {
        if self.col < self.lines[self.row].len() {
            self.col += 1;
        } else if self.row + 1 < self.lines.len() {
            self.row += 1;
            self.col = 0;
            self.ensure_cursor_visible();
        }
    }

    fn page_up(&mut self) {
        let jump = self.visible_height.saturating_sub(2);
        self.row = self.row.saturating_sub(jump);
        self.scroll_offset = self.scroll_offset.saturating_sub(jump);
        self.clamp_col();
    }

    fn page_down(&mut self) {
        let jump = self.visible_height.saturating_sub(2);
        self.row = (self.row + jump).min(self.lines.len().saturating_sub(1));
        self.ensure_cursor_visible();
        self.clamp_col();
    }
}
