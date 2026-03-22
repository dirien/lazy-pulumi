//! Organization selector popup handler

use crossterm::event::KeyEvent;
use std::sync::atomic::Ordering;
use tui_scrollview::ScrollViewState;

use crate::app::App;
use crate::event::keys;

impl App {
    /// Handle organization selector keys
    pub(crate) async fn handle_org_selector_key(&mut self, key: KeyEvent) {
        if keys::is_escape(&key) {
            self.show_org_selector = false;
        } else if keys::is_up(&key) {
            self.org_list.previous();
        } else if keys::is_down(&key) {
            self.org_list.next();
        } else if keys::is_enter(&key) {
            // Select organization and refresh data
            if let Some(org) = self.org_list.selected().cloned() {
                self.state.organization = Some(org.clone());
                self.show_org_selector = false;
                self.is_loading = true;

                // Set the default organization using pulumi CLI (fire-and-forget)
                Self::spawn_set_default_org(org);

                self.spinner.set_message("Loading organization data...");

                // Clear all view-specific state
                self.state.selected_stack_updates.clear();
                self.state.selected_env_yaml = None;
                self.state.selected_env_values = None;
                self.state.neo_messages.clear();
                self.state.current_task_id = None;
                self.neo_scroll_state = ScrollViewState::default();
                self.neo_auto_scroll.store(true, Ordering::Relaxed);

                // Refresh all data for the new organization (non-blocking)
                self.refresh_data();
                // Note: is_loading will be cleared when all spawned tasks complete
            }
        }
    }
}
