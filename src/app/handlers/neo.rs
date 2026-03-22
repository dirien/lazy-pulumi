//! Neo AI agent tab handler

use crossterm::event::KeyEvent;
use std::sync::atomic::Ordering;
use tui_scrollview::ScrollViewState;

use crate::app::types::{FocusMode, SlashCommandsDialogView};
use crate::app::App;
use crate::event::keys;

impl App {
    /// Handle Neo view keys
    pub(crate) async fn handle_neo_key(&mut self, key: KeyEvent) {
        // Esc shows the task list again (if hidden)
        if keys::is_escape(&key) {
            if self.neo_hide_task_list {
                self.neo_hide_task_list = false;
            }
            return;
        }

        if keys::is_char(&key, 'i') {
            self.focus = FocusMode::Input;
            self.neo_input.set_focused(true);
        } else if keys::is_char(&key, '/') {
            // Trigger slash command picker by entering input mode with '/'
            self.neo_input.set_value("/".to_string());
            self.focus = FocusMode::Input;
            self.neo_input.set_focused(true);
            // Show all commands when just '/' is typed
            self.neo_filtered_commands = self.state.neo_slash_commands.clone();
            self.neo_show_command_picker = !self.neo_filtered_commands.is_empty();
            self.neo_command_picker_index = 0;
        } else if keys::is_char(&key, 'n') {
            // Start new task
            self.state.neo_messages.clear();
            self.state.current_task_id = None;
            self.neo_scroll_state = ScrollViewState::default();
            self.neo_auto_scroll.store(true, Ordering::Relaxed);
            self.neo_hide_task_list = true; // Hide task list for new conversation
            self.neo_task_settings = crate::app::types::NeoTaskSettings::default();
            self.focus = FocusMode::Input;
            self.neo_input.set_focused(true);
        } else if keys::is_char(&key, 'a')
            && self.neo_hide_task_list
            && self.state.current_task_id.is_none()
        {
            // Cycle approval mode (only when composing new task)
            self.neo_task_settings.approval_mode = self.neo_task_settings.approval_mode.cycle();
        } else if keys::is_char(&key, 'p')
            && self.neo_hide_task_list
            && self.state.current_task_id.is_none()
        {
            // Cycle permission mode (only when composing new task)
            self.neo_task_settings.permission_mode = self.neo_task_settings.permission_mode.cycle();
        } else if keys::is_char(&key, 'm')
            && self.neo_hide_task_list
            && self.state.current_task_id.is_none()
        {
            // Cycle plan mode (only when composing new task)
            self.neo_task_settings.plan_mode = self.neo_task_settings.plan_mode.cycle();
        } else if keys::is_up(&key) {
            if !self.neo_hide_task_list {
                // Navigate task list when visible and load the selected task
                self.neo_tasks_list.previous();
                self.load_selected_task().await;
            } else {
                // Scroll chat up when in full-width mode
                for _ in 0..3 {
                    self.neo_scroll_state.scroll_up();
                }
                self.neo_auto_scroll.store(false, Ordering::Relaxed);
            }
        } else if keys::is_down(&key) {
            if !self.neo_hide_task_list {
                // Navigate task list when visible and load the selected task
                self.neo_tasks_list.next();
                self.load_selected_task().await;
            } else {
                // Scroll chat down when in full-width mode
                for _ in 0..3 {
                    self.neo_scroll_state.scroll_down();
                }
            }
        } else if keys::is_char(&key, 'k') {
            // Scroll chat up (vim-style) - toward older messages
            for _ in 0..3 {
                self.neo_scroll_state.scroll_up();
            }
            self.neo_auto_scroll.store(false, Ordering::Relaxed);
        } else if keys::is_char(&key, 'j') {
            // Scroll chat down (vim-style) - toward newer messages
            for _ in 0..3 {
                self.neo_scroll_state.scroll_down();
            }
        } else if keys::is_page_up(&key) || keys::is_char(&key, 'K') {
            // Scroll chat up by page
            self.neo_scroll_state.scroll_page_up();
            self.neo_auto_scroll.store(false, Ordering::Relaxed);
        } else if keys::is_page_down(&key) || keys::is_char(&key, 'J') {
            // Scroll chat down by page
            self.neo_scroll_state.scroll_page_down();
        } else if keys::is_char(&key, 'G') {
            // Scroll to bottom (newest messages) - re-enable auto-scroll
            // The render function will handle the actual scroll position
            self.neo_auto_scroll.store(true, Ordering::Relaxed);
        } else if keys::is_char(&key, 'g') {
            // Scroll to top (oldest messages)
            self.neo_scroll_state.scroll_to_top();
            self.neo_auto_scroll.store(false, Ordering::Relaxed);
        } else if keys::is_enter(&key) {
            // Load task and hide task list for full-width chat
            if !self.neo_hide_task_list {
                self.load_selected_task().await;
                self.neo_hide_task_list = true;
            }
        } else if keys::is_char(&key, 'd') {
            // Show task details dialog only when in full-width chat mode (task list hidden)
            if self.neo_hide_task_list && self.state.current_task_id.is_some() {
                // Refresh task details before showing dialog
                self.refresh_current_task_details().await;
                self.show_neo_details = true;
            }
        } else if keys::is_char(&key, 'c') {
            // Show slash commands management dialog
            self.open_slash_commands_dialog().await;
        }
    }

    /// Open the slash commands management dialog
    pub(crate) async fn open_slash_commands_dialog(&mut self) {
        // Initialize the list with current slash commands
        let commands = self.state.neo_slash_commands.clone();
        self.slash_commands_list.set_items(commands);
        if !self.slash_commands_list.items().is_empty() {
            self.slash_commands_list.select(Some(0));
        }

        // Reset dialog state
        self.slash_commands_dialog_view = SlashCommandsDialogView::List;
        self.slash_command_detail = None;
        self.slash_cmd_create_name = crate::components::TextInput::new();
        self.slash_cmd_create_description = crate::components::TextInput::new();
        self.slash_cmd_create_prompt = crate::components::TextEditor::new();
        self.slash_cmd_create_focus = 0;
        self.slash_cmd_detail_scroll = tui_scrollview::ScrollViewState::default();

        self.show_slash_commands_dialog = true;
    }

    /// Handle slash commands dialog key events
    pub(crate) async fn handle_slash_commands_dialog_key(&mut self, key: KeyEvent) {
        match self.slash_commands_dialog_view {
            SlashCommandsDialogView::List => {
                self.handle_slash_commands_list_key(key).await;
            }
            SlashCommandsDialogView::Detail => {
                self.handle_slash_commands_detail_key(key);
            }
            SlashCommandsDialogView::Create => {
                self.handle_slash_commands_create_key(key).await;
            }
            SlashCommandsDialogView::Edit => {
                self.handle_slash_commands_edit_key(key).await;
            }
            SlashCommandsDialogView::ConfirmDelete => {
                self.handle_slash_commands_delete_key(key).await;
            }
        }
    }

    /// Handle keys in slash commands list view
    async fn handle_slash_commands_list_key(&mut self, key: KeyEvent) {
        if keys::is_escape(&key) {
            // Close the dialog
            self.show_slash_commands_dialog = false;
        } else if keys::is_up(&key) {
            self.slash_commands_list.previous();
        } else if keys::is_down(&key) {
            self.slash_commands_list.next();
        } else if keys::is_enter(&key) {
            // View selected command detail
            if let Some(cmd) = self.slash_commands_list.selected().cloned() {
                self.slash_command_detail = Some(cmd);
                self.slash_cmd_detail_scroll = tui_scrollview::ScrollViewState::default();
                self.slash_commands_dialog_view = SlashCommandsDialogView::Detail;
            }
        } else if keys::is_char(&key, 'n') {
            // Create new command
            self.slash_cmd_create_name = crate::components::TextInput::new();
            self.slash_cmd_create_description = crate::components::TextInput::new();
            self.slash_cmd_create_prompt = crate::components::TextEditor::new();
            self.slash_cmd_create_focus = 0;
            self.slash_cmd_create_name.set_focused(true);
            self.slash_commands_dialog_view = SlashCommandsDialogView::Create;
        } else if keys::is_char(&key, 'e') {
            // Edit selected command (only custom commands)
            if let Some(cmd) = self.slash_commands_list.selected().cloned() {
                if cmd.built_in {
                    self.error = Some("Built-in commands cannot be edited".to_string());
                } else {
                    self.start_edit_slash_command(cmd);
                }
            }
        } else if keys::is_char(&key, 'd') {
            // Delete selected command (only custom commands)
            if let Some(cmd) = self.slash_commands_list.selected().cloned() {
                if cmd.built_in {
                    self.error = Some("Built-in commands cannot be deleted".to_string());
                } else {
                    self.slash_command_detail = Some(cmd);
                    self.slash_commands_dialog_view = SlashCommandsDialogView::ConfirmDelete;
                }
            }
        } else if keys::is_home(&key) || keys::is_char(&key, 'g') {
            self.slash_commands_list.select_first();
        } else if keys::is_end(&key) || keys::is_char(&key, 'G') {
            self.slash_commands_list.select_last();
        }
    }

    /// Handle keys in slash command detail view
    fn handle_slash_commands_detail_key(&mut self, key: KeyEvent) {
        if keys::is_escape(&key) {
            // Back to list
            self.slash_commands_dialog_view = SlashCommandsDialogView::List;
            self.slash_command_detail = None;
        } else if keys::is_char(&key, 'j') {
            // Scroll down
            for _ in 0..3 {
                self.slash_cmd_detail_scroll.scroll_down();
            }
        } else if keys::is_char(&key, 'k') {
            // Scroll up
            for _ in 0..3 {
                self.slash_cmd_detail_scroll.scroll_up();
            }
        } else if keys::is_page_down(&key) || keys::is_char(&key, 'J') {
            self.slash_cmd_detail_scroll.scroll_page_down();
        } else if keys::is_page_up(&key) || keys::is_char(&key, 'K') {
            self.slash_cmd_detail_scroll.scroll_page_up();
        } else if keys::is_char(&key, 'e') {
            // Edit the command (only custom commands)
            if let Some(ref cmd) = self.slash_command_detail {
                if cmd.built_in {
                    self.error = Some("Built-in commands cannot be edited".to_string());
                } else {
                    self.start_edit_slash_command(cmd.clone());
                }
            }
        }
    }

    /// Handle keys in slash command create view
    async fn handle_slash_commands_create_key(&mut self, key: KeyEvent) {
        if keys::is_escape(&key) {
            // Cancel and back to list
            self.slash_commands_dialog_view = SlashCommandsDialogView::List;
        } else if keys::is_tab(&key) {
            // Move to next field
            match self.slash_cmd_create_focus {
                0 => {
                    self.slash_cmd_create_name.set_focused(false);
                    self.slash_cmd_create_description.set_focused(true);
                    self.slash_cmd_create_focus = 1;
                }
                1 => {
                    self.slash_cmd_create_description.set_focused(false);
                    self.slash_cmd_create_focus = 2;
                }
                _ => {
                    self.slash_cmd_create_name.set_focused(true);
                    self.slash_cmd_create_focus = 0;
                }
            }
        } else if keys::is_backtab(&key) {
            // Move to previous field
            match self.slash_cmd_create_focus {
                0 => {
                    self.slash_cmd_create_name.set_focused(false);
                    self.slash_cmd_create_focus = 2;
                }
                1 => {
                    self.slash_cmd_create_description.set_focused(false);
                    self.slash_cmd_create_name.set_focused(true);
                    self.slash_cmd_create_focus = 0;
                }
                _ => {
                    self.slash_cmd_create_description.set_focused(true);
                    self.slash_cmd_create_focus = 1;
                }
            }
        } else if keys::is_ctrl_char(&key, 's') {
            // Ctrl+S to save from any field
            self.submit_create_slash_command().await;
        } else {
            // Pass key to focused field (Enter is ignored in single-line inputs, adds newlines in prompt)
            match self.slash_cmd_create_focus {
                0 => {
                    self.slash_cmd_create_name.handle_key(&key);
                }
                1 => {
                    self.slash_cmd_create_description.handle_key(&key);
                }
                _ => {
                    self.slash_cmd_create_prompt.handle_key(&key);
                }
            }
        }
    }

    /// Submit the create slash command form
    async fn submit_create_slash_command(&mut self) {
        let name = self.slash_cmd_create_name.value().trim().to_string();
        let description = self.slash_cmd_create_description.value().trim().to_string();
        let prompt = self.slash_cmd_create_prompt.content().trim().to_string();

        // Validate
        if name.is_empty() {
            self.error = Some("Command name is required".to_string());
            return;
        }

        if description.is_empty() {
            self.error = Some("Description is required".to_string());
            return;
        }

        if prompt.is_empty() {
            self.error = Some("Prompt is required".to_string());
            return;
        }

        // Strip leading slash if present
        let name = name.trim_start_matches('/').to_string();

        // Check for existing command with same name
        if self.state.neo_slash_commands.iter().any(|c| c.name == name) {
            self.error = Some(format!("Command '{}' already exists", name));
            return;
        }

        // Create the command via API
        if let Some(ref client) = self.client {
            if let Some(ref org) = self.state.organization {
                self.is_loading = true;
                self.spinner.set_message("Creating command...");

                match client
                    .create_neo_slash_command(org, &name, &prompt, &description)
                    .await
                {
                    Ok(new_cmd) => {
                        log::info!("Created slash command: /{}", name);

                        // Add to state
                        self.state.neo_slash_commands.push(new_cmd.clone());

                        // Update dialog list
                        let commands = self.state.neo_slash_commands.clone();
                        self.slash_commands_list.set_items(commands);

                        // Select the new command
                        if let Some(idx) = self
                            .slash_commands_list
                            .items()
                            .iter()
                            .position(|c| c.name == name)
                        {
                            self.slash_commands_list.select(Some(idx));
                        }

                        // Back to list view
                        self.slash_commands_dialog_view = SlashCommandsDialogView::List;
                    }
                    Err(e) => {
                        log::error!("Failed to create slash command: {}", e);
                        self.error = Some(format!("Failed to create command: {}", e));
                    }
                }

                self.is_loading = false;
            } else {
                self.error = Some("No organization selected".to_string());
            }
        }
    }

    /// Handle keys in slash command delete confirmation view
    async fn handle_slash_commands_delete_key(&mut self, key: KeyEvent) {
        if keys::is_char(&key, 'y') || keys::is_char(&key, 'Y') {
            // Confirm delete
            if let Some(ref cmd) = self.slash_command_detail {
                if let Some(ref client) = self.client {
                    if let Some(ref org) = self.state.organization {
                        self.is_loading = true;
                        self.spinner.set_message("Deleting command...");

                        let cmd_name = cmd.name.clone();

                        // Fetch fresh command to get the latest tag (avoids conflicts)
                        let fresh_cmd = match client.get_neo_slash_command(org, &cmd_name).await {
                            Ok(c) => c,
                            Err(e) => {
                                log::error!("Failed to fetch fresh command before delete: {}", e);
                                self.error = Some(format!("Failed to fetch command: {}", e));
                                self.is_loading = false;
                                // Back to list
                                self.slash_command_detail = None;
                                self.slash_commands_dialog_view = SlashCommandsDialogView::List;
                                return;
                            }
                        };

                        let cmd_tag = match fresh_cmd.tag {
                            Some(tag) => tag,
                            None => {
                                self.error = Some("Command is missing version tag".to_string());
                                self.is_loading = false;
                                // Back to list
                                self.slash_command_detail = None;
                                self.slash_commands_dialog_view = SlashCommandsDialogView::List;
                                return;
                            }
                        };

                        match client
                            .delete_neo_slash_command(org, &cmd_name, &cmd_tag)
                            .await
                        {
                            Ok(_) => {
                                log::info!("Deleted slash command: /{}", cmd_name);

                                // Remove from state
                                self.state.neo_slash_commands.retain(|c| c.name != cmd_name);

                                // Update dialog list
                                let commands = self.state.neo_slash_commands.clone();
                                self.slash_commands_list.set_items(commands);

                                // Select first item if available
                                if !self.slash_commands_list.items().is_empty() {
                                    self.slash_commands_list.select(Some(0));
                                }
                            }
                            Err(e) => {
                                log::error!("Failed to delete slash command: {}", e);
                                self.error = Some(format!("Failed to delete command: {}", e));
                            }
                        }

                        self.is_loading = false;
                    }
                }
            }

            // Back to list
            self.slash_command_detail = None;
            self.slash_commands_dialog_view = SlashCommandsDialogView::List;
        } else if keys::is_char(&key, 'n') || keys::is_char(&key, 'N') || keys::is_escape(&key) {
            // Cancel delete
            self.slash_command_detail = None;
            self.slash_commands_dialog_view = SlashCommandsDialogView::List;
        }
    }

    /// Start editing a slash command
    fn start_edit_slash_command(&mut self, cmd: crate::api::NeoSlashCommand) {
        // Initialize edit form with current values
        self.slash_cmd_edit_description = crate::components::TextInput::new();
        self.slash_cmd_edit_description
            .set_value(cmd.description.clone());
        self.slash_cmd_edit_prompt = crate::components::TextEditor::with_content(&cmd.prompt);
        self.slash_cmd_edit_focus = 0;
        self.slash_cmd_edit_description.set_focused(true);

        // Store the command being edited
        self.slash_command_detail = Some(cmd);

        // Switch to edit view
        self.slash_commands_dialog_view = SlashCommandsDialogView::Edit;
    }

    /// Handle keys in slash command edit view
    async fn handle_slash_commands_edit_key(&mut self, key: KeyEvent) {
        if keys::is_escape(&key) {
            // Cancel and back to list
            self.slash_commands_dialog_view = SlashCommandsDialogView::List;
            self.slash_command_detail = None;
        } else if keys::is_tab(&key) {
            // Move to next field (only 2 fields: description and prompt)
            match self.slash_cmd_edit_focus {
                0 => {
                    self.slash_cmd_edit_description.set_focused(false);
                    self.slash_cmd_edit_focus = 1;
                }
                _ => {
                    self.slash_cmd_edit_description.set_focused(true);
                    self.slash_cmd_edit_focus = 0;
                }
            }
        } else if keys::is_backtab(&key) {
            // Move to previous field
            match self.slash_cmd_edit_focus {
                0 => {
                    self.slash_cmd_edit_description.set_focused(false);
                    self.slash_cmd_edit_focus = 1;
                }
                _ => {
                    self.slash_cmd_edit_description.set_focused(true);
                    self.slash_cmd_edit_focus = 0;
                }
            }
        } else if keys::is_ctrl_char(&key, 's') {
            // Ctrl+S to save from any field
            self.submit_edit_slash_command().await;
        } else {
            // Pass key to focused field (Enter is ignored in single-line inputs, adds newlines in prompt)
            match self.slash_cmd_edit_focus {
                0 => {
                    self.slash_cmd_edit_description.handle_key(&key);
                }
                _ => {
                    self.slash_cmd_edit_prompt.handle_key(&key);
                }
            }
        }
    }

    /// Submit the edit slash command form
    async fn submit_edit_slash_command(&mut self) {
        let description = self.slash_cmd_edit_description.value().trim().to_string();
        let prompt = self.slash_cmd_edit_prompt.content().trim().to_string();

        // Get the command being edited
        let cmd = match &self.slash_command_detail {
            Some(c) => c.clone(),
            None => {
                self.error = Some("No command selected for editing".to_string());
                return;
            }
        };

        // Validate
        if description.is_empty() {
            self.error = Some("Description is required".to_string());
            return;
        }

        if prompt.is_empty() {
            self.error = Some("Prompt is required".to_string());
            return;
        }

        // Update the command via API
        if let Some(ref client) = self.client {
            if let Some(ref org) = self.state.organization {
                self.is_loading = true;
                self.spinner.set_message("Updating command...");

                let cmd_name = cmd.name.clone();

                // Fetch fresh command to get the latest tag (avoids conflicts)
                let fresh_cmd = match client.get_neo_slash_command(org, &cmd_name).await {
                    Ok(c) => c,
                    Err(e) => {
                        log::error!("Failed to fetch fresh command before update: {}", e);
                        self.error = Some(format!("Failed to fetch command: {}", e));
                        self.is_loading = false;
                        return;
                    }
                };

                let cmd_tag = match fresh_cmd.tag {
                    Some(tag) => tag,
                    None => {
                        self.error = Some("Command is missing version tag".to_string());
                        self.is_loading = false;
                        return;
                    }
                };

                match client
                    .update_neo_slash_command(org, &cmd_name, &prompt, &description, &cmd_tag)
                    .await
                {
                    Ok(updated_cmd) => {
                        log::info!("Updated slash command: /{}", cmd_name);

                        // Update in state
                        if let Some(pos) = self
                            .state
                            .neo_slash_commands
                            .iter()
                            .position(|c| c.name == cmd_name)
                        {
                            self.state.neo_slash_commands[pos] = updated_cmd;
                        }

                        // Update dialog list
                        let commands = self.state.neo_slash_commands.clone();
                        self.slash_commands_list.set_items(commands);

                        // Select the updated command
                        if let Some(idx) = self
                            .slash_commands_list
                            .items()
                            .iter()
                            .position(|c| c.name == cmd_name)
                        {
                            self.slash_commands_list.select(Some(idx));
                        }

                        // Back to list view
                        self.slash_command_detail = None;
                        self.slash_commands_dialog_view = SlashCommandsDialogView::List;
                    }
                    Err(crate::api::ApiError::Conflict) => {
                        log::warn!("Conflict updating slash command - will refresh list");
                        self.error = Some(
                            "Command was modified elsewhere. Refreshing list - please try again."
                                .to_string(),
                        );
                        // Refresh the slash commands list to get updated tags
                        if let Ok(commands) = client.get_neo_slash_commands(org).await {
                            self.state.neo_slash_commands = commands.clone();
                            self.slash_commands_list.set_items(commands);
                        }
                        // Go back to list view
                        self.slash_command_detail = None;
                        self.slash_commands_dialog_view = SlashCommandsDialogView::List;
                    }
                    Err(e) => {
                        log::error!("Failed to update slash command: {}", e);
                        self.error = Some(format!("Failed to update command: {}", e));
                    }
                }

                self.is_loading = false;
            } else {
                self.error = Some("No organization selected".to_string());
            }
        }
    }
}
