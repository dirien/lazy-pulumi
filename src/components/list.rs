//! Stateful list component with selection

use ratatui::widgets::ListState;

/// A list with selection state
#[derive(Debug, Default)]
pub struct StatefulList<T> {
    /// The list state for ratatui
    pub state: ListState,
    /// The items in the list
    items: Vec<T>,
}

impl<T> StatefulList<T> {
    /// Create a new stateful list
    pub fn new() -> Self {
        Self {
            state: ListState::default(),
            items: Vec::new(),
        }
    }

    /// Create with items
    #[allow(dead_code)]
    pub fn with_items(items: Vec<T>) -> Self {
        let mut list = Self::new();
        list.set_items(items);
        list
    }

    /// Get the items
    pub fn items(&self) -> &[T] {
        &self.items
    }

    /// Get mutable access to the items
    pub fn items_mut(&mut self) -> &mut Vec<T> {
        &mut self.items
    }

    /// Set the items
    pub fn set_items(&mut self, items: Vec<T>) {
        self.items = items;
        // Reset selection if list is empty
        if self.items.is_empty() {
            self.state.select(None);
        } else if self.state.selected().is_none() {
            self.state.select(Some(0));
        } else if let Some(i) = self.state.selected() {
            // Keep selection valid
            if i >= self.items.len() {
                self.state.select(Some(self.items.len().saturating_sub(1)));
            }
        }
    }

    /// Get the number of items
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Select the next item
    pub fn next(&mut self) {
        if self.items.is_empty() {
            return;
        }
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.items.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    /// Select the previous item
    pub fn previous(&mut self) {
        if self.items.is_empty() {
            return;
        }
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    /// Get the selected item
    pub fn selected(&self) -> Option<&T> {
        self.state.selected().and_then(|i| self.items.get(i))
    }

    /// Get the selected index
    pub fn selected_index(&self) -> Option<usize> {
        self.state.selected()
    }

    /// Select by index
    pub fn select(&mut self, index: Option<usize>) {
        self.state.select(index);
    }

    /// Select first item
    pub fn select_first(&mut self) {
        if !self.items.is_empty() {
            self.state.select(Some(0));
        }
    }

    /// Select last item
    pub fn select_last(&mut self) {
        if !self.items.is_empty() {
            self.state.select(Some(self.items.len() - 1));
        }
    }

    /// Move selection by a page (positive = down, negative = up)
    #[allow(dead_code)]
    pub fn page(&mut self, direction: i32, page_size: usize) {
        if self.items.is_empty() {
            return;
        }
        let current = self.state.selected().unwrap_or(0);
        let new_index = if direction > 0 {
            (current + page_size).min(self.items.len() - 1)
        } else {
            current.saturating_sub(page_size)
        };
        self.state.select(Some(new_index));
    }

    /// Clear the list
    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.items.clear();
        self.state.select(None);
    }
}

impl<T: Clone> Clone for StatefulList<T> {
    fn clone(&self) -> Self {
        Self {
            state: ListState::default().with_selected(self.state.selected()),
            items: self.items.clone(),
        }
    }
}
