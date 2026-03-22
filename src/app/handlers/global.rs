//! Global key dispatcher and logs handler

use crossterm::event::{KeyCode, KeyEvent};
use tui_logger::TuiWidgetEvent;

use crate::app::types::{FocusMode, Tab};
use crate::app::App;
use crate::event::keys;
use crate::ui::CommandsViewState;

impl App {
    /// Handle key events
    pub(crate) async fn handle_key(&mut self, key: KeyEvent) {
        // Handle splash screen first
        if self.show_splash {
            self.handle_splash_key(key);
            return;
        }

        // Handle error dismissal first
        if self.error.is_some() {
            if keys::is_escape(&key) || keys::is_enter(&key) {
                self.error = None;
            }
            return;
        }

        // Handle help popup
        if self.show_help {
            if keys::is_escape(&key) || keys::is_char(&key, '?') {
                self.show_help = false;
            }
            return;
        }

        // Handle Neo details popup
        if self.show_neo_details {
            if keys::is_escape(&key) || keys::is_char(&key, 'd') {
                self.show_neo_details = false;
            }
            return;
        }

        // Handle slash commands management dialog
        if self.show_slash_commands_dialog {
            self.handle_slash_commands_dialog_key(key).await;
            return;
        }

        // Handle ESC YAML editor popup
        if self.show_esc_editor {
            self.handle_esc_editor_key(key).await;
            return;
        }

        // Handle logs popup
        if self.show_logs {
            self.handle_logs_key(key);
            return;
        }

        // Handle organization selector popup
        if self.show_org_selector {
            self.handle_org_selector_key(key).await;
            return;
        }

        // Handle input mode (Neo tab with command picker support)
        if self.focus == FocusMode::Input {
            if keys::is_escape(&key) {
                self.focus = FocusMode::Normal;
                self.neo_input.set_focused(false);
                self.neo_show_command_picker = false;
                self.neo_filtered_commands.clear();
            } else if keys::is_enter(&key) {
                // If command picker is showing, insert the selected command (don't execute yet)
                if self.neo_show_command_picker && !self.neo_filtered_commands.is_empty() {
                    self.insert_selected_slash_command();
                } else {
                    // Send message (may contain slash commands)
                    self.send_neo_message();
                }
            } else if self.neo_show_command_picker {
                // Handle command picker navigation
                if keys::is_up(&key) || (keys::is_ctrl_char(&key, 'p')) {
                    if self.neo_command_picker_index > 0 {
                        self.neo_command_picker_index -= 1;
                    } else if !self.neo_filtered_commands.is_empty() {
                        self.neo_command_picker_index = self.neo_filtered_commands.len() - 1;
                    }
                } else if keys::is_down(&key) || (keys::is_ctrl_char(&key, 'n')) {
                    if self.neo_command_picker_index + 1 < self.neo_filtered_commands.len() {
                        self.neo_command_picker_index += 1;
                    } else {
                        self.neo_command_picker_index = 0;
                    }
                } else if keys::is_tab(&key) {
                    // Tab inserts the command (same as Enter)
                    self.insert_selected_slash_command();
                } else {
                    // Let input handle the key, then update filtered commands
                    self.neo_input.handle_key(&key);
                    self.update_filtered_commands();
                }
            } else {
                // Normal input mode - handle key and check for command trigger
                self.neo_input.handle_key(&key);
                self.update_filtered_commands();
            }
            return;
        }

        // Handle Commands view dialogs before ANY global keys
        // This ensures all keypresses go to the dialog inputs, not global handlers
        if self.tab == Tab::Commands {
            match self.commands_view_state {
                CommandsViewState::InputDialog | CommandsViewState::ConfirmDialog => {
                    self.handle_commands_key(key).await;
                    return;
                }
                _ => {}
            }
        }

        // Global keys
        if keys::is_quit(&key) {
            self.should_quit = true;
            return;
        }

        if keys::is_char(&key, '?') {
            self.show_help = true;
            return;
        }

        // Open logs viewer with 'l'
        if keys::is_char(&key, 'l') {
            self.show_logs = true;
            return;
        }

        // Open organization selector with 'o' (but not in ESC tab where 'o' opens environments)
        // In ESC tab, use 'O' (uppercase) instead
        if (keys::is_char(&key, 'o') && self.tab != Tab::Esc)
            || (keys::is_char(&key, 'O') && self.tab == Tab::Esc)
        {
            self.show_org_selector = true;
            // Select current org in list if present
            if let Some(ref current_org) = self.state.organization {
                if let Some(idx) = self.org_list.items().iter().position(|o| o == current_org) {
                    self.org_list.select(Some(idx));
                }
            }
            return;
        }

        if keys::is_tab(&key) {
            let old_tab = self.tab;
            self.tab = self.tab.next();
            // When switching to Neo tab, show task list unless there's an active task
            if self.tab == Tab::Neo && old_tab != Tab::Neo && self.state.current_task_id.is_none() {
                self.neo_hide_task_list = false;
            }
            return;
        }

        if keys::is_backtab(&key) {
            let old_tab = self.tab;
            self.tab = self.tab.previous();
            // When switching to Neo tab, show task list unless there's an active task
            if self.tab == Tab::Neo && old_tab != Tab::Neo && self.state.current_task_id.is_none() {
                self.neo_hide_task_list = false;
            }
            return;
        }

        if keys::is_char(&key, 'r') {
            // refresh_data sets is_loading and spawns async tasks
            self.refresh_data();
            return;
        }

        // Tab-specific keys
        match self.tab {
            Tab::Dashboard => {
                // Dashboard doesn't need special handling
            }
            Tab::Stacks => {
                self.handle_stacks_key(key).await;
            }
            Tab::Esc => {
                self.handle_esc_key(key).await;
            }
            Tab::Neo => {
                self.handle_neo_key(key).await;
            }
            Tab::Platform => {
                self.handle_platform_key(key).await;
            }
            Tab::Commands => {
                self.handle_commands_key(key).await;
            }
        }
    }

    /// Handle logs popup keys
    /// Maps keys to TuiWidgetEvent for the tui-logger smart widget
    pub(crate) fn handle_logs_key(&mut self, key: KeyEvent) {
        // Close popup
        if keys::is_escape(&key) || keys::is_char(&key, 'l') {
            self.show_logs = false;
            return;
        }

        // Map keys to TuiWidgetEvent
        let event = match key.code {
            // h: Toggle target selector widget hidden/visible
            KeyCode::Char('h') => Some(TuiWidgetEvent::HideKey),
            // f: Toggle focus on selected target only
            KeyCode::Char('f') => Some(TuiWidgetEvent::FocusKey),
            // UP: Select previous target in target selector
            KeyCode::Up => Some(TuiWidgetEvent::UpKey),
            // DOWN: Select next target in target selector
            KeyCode::Down => Some(TuiWidgetEvent::DownKey),
            // LEFT or '<': Reduce SHOWN log messages by one level
            KeyCode::Left | KeyCode::Char('<') => Some(TuiWidgetEvent::LeftKey),
            // RIGHT or '>': Increase SHOWN log messages by one level
            KeyCode::Right | KeyCode::Char('>') => Some(TuiWidgetEvent::RightKey),
            // '-': Reduce CAPTURED log messages by one level
            KeyCode::Char('-') => Some(TuiWidgetEvent::MinusKey),
            // '+' or '=': Increase CAPTURED log messages by one level
            KeyCode::Char('+') | KeyCode::Char('=') => Some(TuiWidgetEvent::PlusKey),
            // PAGEUP: Enter page mode and scroll up in log history
            KeyCode::PageUp => Some(TuiWidgetEvent::PrevPageKey),
            // PAGEDOWN: Scroll down in log history (only in page mode)
            KeyCode::PageDown => Some(TuiWidgetEvent::NextPageKey),
            // SPACE: Toggle hiding of targets with logfilter set to off
            KeyCode::Char(' ') => Some(TuiWidgetEvent::SpaceKey),
            // ESC handled above for closing
            _ => None,
        };

        if let Some(evt) = event {
            self.logger_state.transition(evt);
        }
    }
}
