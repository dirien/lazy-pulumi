//! Stacks tab handler

use crossterm::event::KeyEvent;

use crate::app::App;
use crate::event::keys;

impl App {
    /// Handle stacks view keys
    pub(crate) async fn handle_stacks_key(&mut self, key: KeyEvent) {
        if keys::is_up(&key) {
            self.stacks_list.previous();
            self.state.selected_stack_updates.clear();
        } else if keys::is_down(&key) {
            self.stacks_list.next();
            self.state.selected_stack_updates.clear();
        } else if keys::is_home(&key) || keys::is_char(&key, 'g') {
            self.stacks_list.select_first();
        } else if keys::is_end(&key) || keys::is_char(&key, 'G') {
            self.stacks_list.select_last();
        } else if keys::is_enter(&key) || keys::is_char(&key, 'u') {
            // Load stack updates
            if let Some(stack) = self.stacks_list.selected() {
                if let Some(ref client) = self.client {
                    self.is_loading = true;
                    self.spinner.set_message("Loading updates...");

                    if let Ok(updates) = client
                        .get_stack_updates(&stack.org_name, &stack.project_name, &stack.stack_name)
                        .await
                    {
                        self.state.selected_stack_updates = updates
                            .into_iter()
                            .take(10)
                            .map(|u| {
                                let time = u
                                    .start_time
                                    .map(|t| {
                                        chrono::DateTime::from_timestamp(t, 0)
                                            .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
                                            .unwrap_or_else(|| "Unknown".to_string())
                                    })
                                    .unwrap_or_else(|| "Unknown".to_string());

                                (
                                    u.version,
                                    u.result.unwrap_or_else(|| "Unknown".to_string()),
                                    time,
                                )
                            })
                            .collect();
                    }

                    self.is_loading = false;
                }
            }
        }
    }
}
