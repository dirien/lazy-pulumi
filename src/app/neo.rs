//! Neo AI agent async operations
//!
//! This module handles all Neo-specific async operations including
//! polling for task updates, processing results, and sending messages.

use std::sync::atomic::Ordering;

use crate::api::{NeoMessage, NeoMessageType, NeoTask};

use super::types::NeoAsyncResult;
use super::App;

impl App {
    /// Process any pending async Neo results
    pub(super) fn process_neo_results(&mut self) {
        // Try to receive all pending results without blocking
        while let Ok(result) = self.neo_result_rx.try_recv() {
            match result {
                NeoAsyncResult::TaskCreated { task_id } => {
                    self.state.current_task_id = Some(task_id.clone());
                    // Add new task to list if not already there
                    if !self.state.neo_tasks.iter().any(|t| t.id == task_id) {
                        let msg_preview = self
                            .state
                            .neo_messages
                            .iter()
                            .find(|m| m.message_type == NeoMessageType::UserMessage)
                            .map(|m| {
                                let s: String = m.content.chars().take(50).collect();
                                if m.content.len() > 50 {
                                    format!("{}...", s)
                                } else {
                                    s
                                }
                            })
                            .unwrap_or_else(|| "New task".to_string());

                        let new_task = NeoTask {
                            id: task_id,
                            name: Some(msg_preview),
                            status: Some("running".to_string()),
                            created_at: Some(chrono::Utc::now().to_rfc3339()),
                            updated_at: None,
                            url: None,
                            started_by: None,
                            linked_prs: Vec::new(),
                            entities: Vec::new(),
                            policies: Vec::new(),
                        };
                        self.state.neo_tasks.insert(0, new_task);
                        self.neo_tasks_list.set_items(self.state.neo_tasks.clone());
                        self.neo_tasks_list.select(Some(0));
                    }
                    // Start polling for updates
                    self.neo_polling = true;
                    self.neo_poll_counter = 5; // Trigger immediate poll on next tick
                }
                NeoAsyncResult::EventsReceived { messages, has_more: _, task_status } => {
                    let current_count = messages.len();

                    // Only update if we got messages from the API
                    if !messages.is_empty() {
                        // Check if this is actually new content
                        let has_new_content = current_count != self.state.neo_messages.len()
                            || messages.iter().any(|m| {
                                !self.state.neo_messages.iter().any(|existing| {
                                    existing.content == m.content
                                        && existing.message_type == m.message_type
                                })
                            });

                        if has_new_content {
                            self.state.neo_messages = messages;
                            // Auto-scroll is handled by the render function
                            // Reset stable counter since we got new content
                            self.neo_stable_polls = 0;
                        } else {
                            self.neo_stable_polls += 1;
                        }
                    } else {
                        self.neo_stable_polls += 1;
                    }

                    // Increment poll count
                    self.neo_current_poll += 1;
                    self.neo_prev_message_count = current_count;

                    // Check task status - is NEO still working?
                    let task_is_running = task_status
                        .as_ref()
                        .map(|s| {
                            let s_lower = s.to_lowercase();
                            s_lower == "running" || s_lower == "in_progress" || s_lower == "pending"
                        })
                        .unwrap_or(false);

                    // Update the task running state - this keeps the thinking indicator visible
                    // until we confirm the task is no longer running
                    self.neo_task_is_running = task_is_running;

                    // Check for assistant response
                    let has_assistant_response =
                        self.state.neo_messages.iter().any(|m| {
                            m.message_type == NeoMessageType::AssistantMessage && !m.content.is_empty()
                        });

                    // Stop polling if:
                    // 1. Task status is NOT running/in_progress (i.e., idle, completed, failed)
                    //    AND we have at least one assistant message
                    // 2. OR we've hit max polls (safety timeout)
                    // 3. OR stable polls exceeded AND task is not running (fallback for API issues)
                    let should_stop = (!task_is_running && has_assistant_response)
                        || self.neo_current_poll >= self.neo_max_polls
                        || (self.neo_stable_polls >= 20 && !task_is_running);

                    log::debug!(
                        "Neo poll: status={:?}, running={}, stable={}, poll={}/{}, stop={}",
                        task_status, task_is_running, self.neo_stable_polls,
                        self.neo_current_poll, self.neo_max_polls, should_stop
                    );

                    if should_stop {
                        self.neo_polling = false;
                        self.is_loading = false;
                        // Reset poll counters
                        self.neo_stable_polls = 0;
                        self.neo_prev_message_count = 0;
                        self.neo_current_poll = 0;
                        // Note: neo_task_is_running is already set above based on task_status
                        // so the thinking indicator will stay visible if task is still running
                    }
                }
                NeoAsyncResult::Error(e) => {
                    self.error = Some(format!("Neo error: {}", e));
                    self.neo_polling = false;
                    self.is_loading = false;
                    self.neo_task_is_running = false;
                    // Reset poll counters
                    self.neo_stable_polls = 0;
                    self.neo_prev_message_count = 0;
                    self.neo_current_poll = 0;
                }
            }
        }
    }

    /// Spawn async task to poll Neo events and task status
    pub(super) fn spawn_neo_poll(&self) {
        if let (Some(task_id), Some(org)) =
            (&self.state.current_task_id, &self.state.organization)
        {
            if let Some(ref client) = self.client {
                let client = client.clone();
                let task_id = task_id.clone();
                let org = org.clone();
                let tx = self.neo_result_tx.clone();

                tokio::spawn(async move {
                    // Fetch both events and task status in parallel
                    let (events_result, task_result) = tokio::join!(
                        client.get_neo_task_events(&org, &task_id),
                        client.get_neo_task(&org, &task_id)
                    );

                    // Extract task status (if available)
                    let task_status = task_result
                        .ok()
                        .and_then(|task| task.status);

                    match events_result {
                        Ok(response) => {
                            let _ = tx
                                .send(NeoAsyncResult::EventsReceived {
                                    messages: response.messages,
                                    has_more: response.has_more,
                                    task_status,
                                })
                                .await;
                        }
                        Err(e) => {
                            log::warn!("Failed to poll Neo task: {}", e);
                            // Don't send error for transient poll failures
                        }
                    }
                });
            }
        }
    }

    /// Send a message to Neo (non-blocking)
    /// If pending_commands is not empty, sends as slash command payload
    pub(super) fn send_neo_message(&mut self) {
        let message = self.neo_input.take();
        if message.trim().is_empty() {
            return;
        }

        // Take pending commands (they'll be sent with this message)
        let pending_commands = std::mem::take(&mut self.neo_pending_commands);

        // Add user message to chat immediately
        self.state.neo_messages.push(NeoMessage {
            role: "user".to_string(),
            content: message.clone(),
            message_type: NeoMessageType::UserMessage,
            timestamp: None,
            tool_calls: vec![],
            tool_name: None,
        });

        // Auto-scroll is handled by the render function

        self.focus = super::types::FocusMode::Normal;
        self.neo_input.set_focused(false);
        self.is_loading = true;
        self.spinner.set_message("Neo is thinking...");

        // Spawn async task to send message
        if let Some(ref client) = self.client {
            if let Some(org) = &self.state.organization {
                let client = client.clone();
                let org = org.clone();
                let message = message.clone();
                let task_id = self.state.current_task_id.clone();
                let tx = self.neo_result_tx.clone();

                tokio::spawn(async move {
                    let result = if let Some(tid) = task_id {
                        // Continue existing task
                        if !pending_commands.is_empty() {
                            // With slash commands
                            client
                                .continue_neo_task_with_commands(
                                    &org,
                                    &tid,
                                    &message,
                                    &pending_commands,
                                )
                                .await
                        } else {
                            // Plain message
                            client.continue_neo_task(&org, &tid, Some(&message)).await
                        }
                    } else if !pending_commands.is_empty() {
                        // Create new task with slash commands
                        client
                            .create_neo_task_with_commands(&org, &message, &pending_commands)
                            .await
                    } else {
                        // Create new task (plain message)
                        client.create_neo_task(&org, &message).await
                    };

                    match result {
                        Ok(response) => {
                            // Send task created result
                            let _ = tx
                                .send(NeoAsyncResult::TaskCreated {
                                    task_id: response.task_id,
                                })
                                .await;
                        }
                        Err(e) => {
                            let _ = tx.send(NeoAsyncResult::Error(e.to_string())).await;
                        }
                    }
                });

                // Start polling immediately (will pick up results)
                self.neo_polling = true;
                self.neo_poll_counter = 0;
                self.neo_stable_polls = 0;
                self.neo_prev_message_count = self.state.neo_messages.len();
                self.neo_current_poll = 0;
                // Mark task as running - will be updated by polling
                self.neo_task_is_running = true;
                // Enable auto-scroll - render function will handle positioning
                self.neo_auto_scroll.store(true, Ordering::Relaxed);
            }
        }
    }

    /// Load selected Neo task
    pub(super) async fn load_selected_task(&mut self) {
        if let Some(task) = self.neo_tasks_list.selected() {
            self.state.current_task_id = Some(task.id.clone());
            self.state.neo_messages.clear();
            self.neo_scroll_state = tui_scrollview::ScrollViewState::default();
            self.neo_auto_scroll.store(true, Ordering::Relaxed);
            // Reset background poll counter to start fresh polling cycle
            self.neo_bg_poll_counter = 0;
            // Reset task running state - will be updated by polling
            self.neo_task_is_running = false;

            // Try to continue/poll the task to get messages
            if let Some(ref client) = self.client {
                if let Some(org) = &self.state.organization {
                    self.is_loading = true;

                    if let Ok(response) = client.continue_neo_task(org, &task.id, None).await {
                        self.state.neo_messages = response.messages;
                        // Auto-scroll is handled by the render function
                    }

                    self.is_loading = false;
                }
            }
        }
    }

    /// Update filtered commands based on current input
    pub(super) fn update_filtered_commands(&mut self) {
        let input = self.neo_input.value();

        // Find the last '/' that might start a new command
        // This allows typing text after an inserted command, then starting a new one
        let last_slash_pos = input.rfind('/');

        match last_slash_pos {
            Some(pos) => {
                // Check if there's a space after this slash (command already completed)
                let after_slash = &input[pos + 1..];
                if after_slash.contains(' ') {
                    // Command is complete (has space after), hide picker
                    self.neo_show_command_picker = false;
                    self.neo_filtered_commands.clear();
                    return;
                }

                // Get the filter text (everything after the last /)
                let filter = after_slash.to_lowercase();

                // Filter commands that match
                self.neo_filtered_commands = self
                    .state
                    .neo_slash_commands
                    .iter()
                    .filter(|cmd| {
                        cmd.name.to_lowercase().contains(&filter)
                            || cmd.description.to_lowercase().contains(&filter)
                    })
                    .cloned()
                    .collect();

                // Show picker if we have matches
                self.neo_show_command_picker = !self.neo_filtered_commands.is_empty();

                // Reset selection index if out of bounds
                if self.neo_command_picker_index >= self.neo_filtered_commands.len() {
                    self.neo_command_picker_index = 0;
                }
            }
            None => {
                self.neo_show_command_picker = false;
                self.neo_filtered_commands.clear();
            }
        }
    }

    /// Insert the selected slash command into the input (without executing)
    pub(super) fn insert_selected_slash_command(&mut self) {
        if let Some(cmd) = self.neo_filtered_commands.get(self.neo_command_picker_index) {
            let current_input = self.neo_input.value().to_string();

            // Find the last '/' to replace partial command
            if let Some(last_slash_pos) = current_input.rfind('/') {
                // Replace from the last '/' with the full command name
                let prefix = &current_input[..last_slash_pos];
                let new_value = format!("{}/{} ", prefix, cmd.name);
                self.neo_input.set_value(new_value);

                // Track the inserted command for later use when sending
                self.neo_pending_commands.push(cmd.clone());
            } else {
                // No slash found, just set the command
                self.neo_input.set_value(format!("/{} ", cmd.name));
                self.neo_pending_commands.push(cmd.clone());
            }

            // Hide picker after insertion
            self.neo_show_command_picker = false;
            self.neo_filtered_commands.clear();
            self.neo_command_picker_index = 0;
        }
    }

    /// Refresh current task details from the API
    pub(super) async fn refresh_current_task_details(&mut self) {
        let task_id = match &self.state.current_task_id {
            Some(id) => id.clone(),
            None => return,
        };

        if let Some(ref client) = self.client {
            if let Some(org) = &self.state.organization {
                // Fetch task metadata using dedicated endpoint (more efficient than listing all tasks)
                if let Ok(updated_task) = client.get_neo_task(org, &task_id).await {
                    // Update the task in our local state
                    if let Some(local_task) =
                        self.state.neo_tasks.iter_mut().find(|t| t.id == task_id)
                    {
                        *local_task = updated_task.clone();
                    }
                    // Also update the tasks list
                    self.neo_tasks_list.set_items(self.state.neo_tasks.clone());
                }
            }
        }
    }
}
