//! Key event handlers
//!
//! This module contains all keyboard event handling logic for the application,
//! organized by context (global, tab-specific, popup-specific).

use crossterm::event::{KeyCode, KeyEvent};
use std::sync::atomic::Ordering;
use tui_logger::TuiWidgetEvent;
use tui_scrollview::ScrollViewState;

use crate::event::keys;
use crate::startup::{check_pulumi_cli, check_pulumi_token, CheckStatus};
use crate::ui::syntax::highlight_yaml;

use super::types::{FocusMode, PlatformView, Tab};
use super::App;
use crate::commands::{
    can_run_command, commands_by_category, spawn_command, CommandExecution, CommandExecutionState,
    ExecutionMode,
};
use crate::ui::{extract_values, json_to_yaml, CommandsViewState};

impl App {
    /// Handle key events
    pub(super) async fn handle_key(&mut self, key: KeyEvent) {
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
    fn handle_logs_key(&mut self, key: KeyEvent) {
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

    /// Handle organization selector keys
    async fn handle_org_selector_key(&mut self, key: KeyEvent) {
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

    /// Handle stacks view keys
    async fn handle_stacks_key(&mut self, key: KeyEvent) {
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

    /// Handle ESC view keys
    async fn handle_esc_key(&mut self, key: KeyEvent) {
        use super::types::EscPane;

        // Left/Right arrows switch between Definition and Resolved Values panes
        if keys::is_left(&key) || keys::is_char(&key, 'h') {
            self.esc_pane = EscPane::Definition;
        } else if keys::is_right(&key) || keys::is_char(&key, 'l') {
            self.esc_pane = EscPane::ResolvedValues;
        }
        // j/k scroll the focused pane
        else if keys::is_char(&key, 'j') {
            match self.esc_pane {
                EscPane::Definition => {
                    for _ in 0..3 {
                        self.esc_definition_scroll.scroll_down();
                    }
                }
                EscPane::ResolvedValues => {
                    for _ in 0..3 {
                        self.esc_values_scroll.scroll_down();
                    }
                }
            }
        } else if keys::is_char(&key, 'k') {
            match self.esc_pane {
                EscPane::Definition => {
                    for _ in 0..3 {
                        self.esc_definition_scroll.scroll_up();
                    }
                }
                EscPane::ResolvedValues => {
                    for _ in 0..3 {
                        self.esc_values_scroll.scroll_up();
                    }
                }
            }
        }
        // J/K page scroll
        else if keys::is_char(&key, 'J') || keys::is_page_down(&key) {
            match self.esc_pane {
                EscPane::Definition => self.esc_definition_scroll.scroll_page_down(),
                EscPane::ResolvedValues => self.esc_values_scroll.scroll_page_down(),
            }
        } else if keys::is_char(&key, 'K') || keys::is_page_up(&key) {
            match self.esc_pane {
                EscPane::Definition => self.esc_definition_scroll.scroll_page_up(),
                EscPane::ResolvedValues => self.esc_values_scroll.scroll_page_up(),
            }
        }
        // Up/Down arrows navigate environment list
        else if keys::is_up(&key) {
            self.esc_list.previous();
            self.state.selected_env_yaml = None;
            self.state.selected_env_yaml_highlighted = None;
            self.state.selected_env_values = None;
            self.state.selected_env_values_highlighted = None;
            // Reset scroll when changing environments
            self.esc_definition_scroll = ScrollViewState::default();
            self.esc_values_scroll = ScrollViewState::default();
        } else if keys::is_down(&key) {
            self.esc_list.next();
            self.state.selected_env_yaml = None;
            self.state.selected_env_yaml_highlighted = None;
            self.state.selected_env_values = None;
            self.state.selected_env_values_highlighted = None;
            // Reset scroll when changing environments
            self.esc_definition_scroll = ScrollViewState::default();
            self.esc_values_scroll = ScrollViewState::default();
        } else if keys::is_home(&key) || keys::is_char(&key, 'g') {
            self.esc_list.select_first();
            self.esc_definition_scroll = ScrollViewState::default();
            self.esc_values_scroll = ScrollViewState::default();
        } else if keys::is_end(&key) || keys::is_char(&key, 'G') {
            self.esc_list.select_last();
            self.esc_definition_scroll = ScrollViewState::default();
            self.esc_values_scroll = ScrollViewState::default();
        } else if keys::is_enter(&key) {
            // Load environment definition
            if let Some(env) = self.esc_list.selected() {
                if let Some(ref client) = self.client {
                    self.is_loading = true;
                    self.spinner.set_message("Loading definition...");

                    log::debug!(
                        "Loading ESC environment definition: org={}, project={}, name={}",
                        env.organization,
                        env.project,
                        env.name
                    );

                    match client
                        .get_esc_environment(&env.organization, &env.project, &env.name)
                        .await
                    {
                        Ok(details) => {
                            // Cache syntax-highlighted content when loading (not on every render)
                            self.state.selected_env_yaml_highlighted =
                                details.yaml.as_ref().map(|y| highlight_yaml(y));
                            self.state.selected_env_yaml = details.yaml;
                            self.esc_definition_scroll = ScrollViewState::default();
                            log::debug!("ESC environment definition loaded successfully");
                        }
                        Err(e) => {
                            log::error!("Failed to load ESC environment definition: {}", e);
                            self.error = Some(format!("Failed to load definition: {}", e));
                        }
                    }

                    self.is_loading = false;
                }
            }
        } else if keys::is_char(&key, 'o') {
            // Open and resolve environment
            if let Some(env) = self.esc_list.selected() {
                if let Some(ref client) = self.client {
                    self.is_loading = true;
                    self.spinner.set_message("Opening environment...");

                    log::debug!(
                        "Opening ESC environment: org={}, project={}, name={}",
                        env.organization,
                        env.project,
                        env.name
                    );

                    match client
                        .open_esc_environment(&env.organization, &env.project, &env.name)
                        .await
                    {
                        Ok(response) => {
                            // Cache syntax-highlighted content when loading (not on every render)
                            self.state.selected_env_values_highlighted =
                                response.values.as_ref().map(|v| {
                                    let filtered = extract_values(v);
                                    let yaml_str = json_to_yaml(&filtered);
                                    highlight_yaml(&yaml_str)
                                });
                            self.state.selected_env_values = response.values;
                            self.esc_values_scroll = ScrollViewState::default();
                            log::debug!("ESC environment opened and resolved successfully");
                        }
                        Err(e) => {
                            log::error!("Failed to open ESC environment: {}", e);
                            self.error = Some(format!("Failed to open environment: {}", e));
                        }
                    }

                    self.is_loading = false;
                }
            }
        } else if keys::is_char(&key, 'e') {
            // Edit environment definition in YAML editor
            // First, ensure we have the definition loaded
            if let Some(env) = self.esc_list.selected() {
                let yaml_content = if let Some(ref yaml) = self.state.selected_env_yaml {
                    yaml.clone()
                } else {
                    // Need to load it first
                    if let Some(ref client) = self.client {
                        self.is_loading = true;
                        self.spinner.set_message("Loading definition...");

                        match client
                            .get_esc_environment(&env.organization, &env.project, &env.name)
                            .await
                        {
                            Ok(details) => {
                                self.state.selected_env_yaml = details.yaml.clone();
                                details.yaml.unwrap_or_default()
                            }
                            Err(e) => {
                                log::error!("Failed to load ESC environment definition: {}", e);
                                self.error = Some(format!("Failed to load definition: {}", e));
                                self.is_loading = false;
                                return;
                            }
                        }
                    } else {
                        return;
                    }
                };

                self.is_loading = false;

                // Initialize editor with content
                self.esc_editor = crate::components::TextEditor::with_content(&yaml_content);
                self.esc_editing_env = Some((
                    env.organization.clone(),
                    env.project.clone(),
                    env.name.clone(),
                ));
                self.show_esc_editor = true;
            }
        }
    }

    /// Handle ESC YAML editor keys
    async fn handle_esc_editor_key(&mut self, key: KeyEvent) {
        // Escape = Save and close
        if keys::is_escape(&key) {
            if self.esc_editor.is_modified() {
                // Save the content
                if let Some((ref org, ref project, ref env_name)) = self.esc_editing_env {
                    if let Some(ref client) = self.client {
                        self.is_loading = true;
                        self.spinner.set_message("Saving environment...");

                        let content = self.esc_editor.content();

                        match client
                            .update_esc_environment(org, project, env_name, &content)
                            .await
                        {
                            Ok(_) => {
                                log::info!("ESC environment saved successfully");
                                // Update the cached YAML
                                self.state.selected_env_yaml = Some(content);
                            }
                            Err(e) => {
                                log::error!("Failed to save ESC environment: {}", e);
                                self.error = Some(format!("Failed to save: {}", e));
                            }
                        }

                        self.is_loading = false;
                    }
                }
            }

            self.show_esc_editor = false;
            self.esc_editing_env = None;
            return;
        }

        // Ctrl+C = Cancel without saving
        if keys::is_ctrl_char(&key, 'c') {
            self.show_esc_editor = false;
            self.esc_editing_env = None;
            return;
        }

        // Let the editor handle all other keys
        self.esc_editor.handle_key(&key);
    }

    /// Handle Neo view keys
    async fn handle_neo_key(&mut self, key: KeyEvent) {
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
            self.focus = FocusMode::Input;
            self.neo_input.set_focused(true);
        } else if keys::is_up(&key) {
            if !self.neo_hide_task_list {
                // Navigate task list when visible
                self.neo_tasks_list.previous();
            } else {
                // Scroll chat up when in full-width mode
                for _ in 0..3 {
                    self.neo_scroll_state.scroll_up();
                }
                self.neo_auto_scroll.store(false, Ordering::Relaxed);
            }
        } else if keys::is_down(&key) {
            if !self.neo_hide_task_list {
                // Navigate task list when visible
                self.neo_tasks_list.next();
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
        }
    }

    /// Handle Platform view keys
    async fn handle_platform_key(&mut self, key: KeyEvent) {
        // For Components/Templates views: j/k scroll description, arrow keys navigate list
        // For Services view: both j/k and arrow keys navigate list
        match key.code {
            // j/k keys - scroll description in Components/Templates, navigate list in Services
            KeyCode::Char('j') => match self.platform_view {
                PlatformView::Services => self.services_list.next(),
                PlatformView::Components | PlatformView::Templates => {
                    self.platform_desc_scroll_state.scroll_down();
                }
            },
            KeyCode::Char('k') => match self.platform_view {
                PlatformView::Services => self.services_list.previous(),
                PlatformView::Components | PlatformView::Templates => {
                    self.platform_desc_scroll_state.scroll_up();
                }
            },
            // J/K for page scroll in description
            KeyCode::Char('J') => match self.platform_view {
                PlatformView::Services => {}
                PlatformView::Components | PlatformView::Templates => {
                    self.platform_desc_scroll_state.scroll_page_down();
                }
            },
            KeyCode::Char('K') => match self.platform_view {
                PlatformView::Services => {}
                PlatformView::Components | PlatformView::Templates => {
                    self.platform_desc_scroll_state.scroll_page_up();
                }
            },
            // Arrow keys - always navigate the list
            KeyCode::Up => match self.platform_view {
                PlatformView::Services => self.services_list.previous(),
                PlatformView::Components => {
                    self.packages_list.previous();
                    self.platform_desc_scroll_state = ScrollViewState::default();
                    self.spawn_readme_load_for_selected_package();
                }
                PlatformView::Templates => {
                    self.templates_list.previous();
                    self.platform_desc_scroll_state = ScrollViewState::default();
                }
            },
            KeyCode::Down => match self.platform_view {
                PlatformView::Services => self.services_list.next(),
                PlatformView::Components => {
                    self.packages_list.next();
                    self.platform_desc_scroll_state = ScrollViewState::default();
                    self.spawn_readme_load_for_selected_package();
                }
                PlatformView::Templates => {
                    self.templates_list.next();
                    self.platform_desc_scroll_state = ScrollViewState::default();
                }
            },
            // Left/Right and h/l - switch between views
            KeyCode::Left | KeyCode::Char('h') => {
                self.platform_view = self.platform_view.previous();
                self.platform_desc_scroll_state = ScrollViewState::default();
                if self.platform_view == PlatformView::Components {
                    self.spawn_readme_load_for_selected_package();
                }
            }
            KeyCode::Right | KeyCode::Char('l') => {
                self.platform_view = self.platform_view.next();
                self.platform_desc_scroll_state = ScrollViewState::default();
                if self.platform_view == PlatformView::Components {
                    self.spawn_readme_load_for_selected_package();
                }
            }
            // PageUp/PageDown - page scroll description
            KeyCode::PageUp => match self.platform_view {
                PlatformView::Services => {}
                PlatformView::Components | PlatformView::Templates => {
                    self.platform_desc_scroll_state.scroll_page_up();
                }
            },
            KeyCode::PageDown => match self.platform_view {
                PlatformView::Services => {}
                PlatformView::Components | PlatformView::Templates => {
                    self.platform_desc_scroll_state.scroll_page_down();
                }
            },
            // Home/g - go to first item
            KeyCode::Home | KeyCode::Char('g') => match self.platform_view {
                PlatformView::Services => self.services_list.select_first(),
                PlatformView::Components => {
                    self.packages_list.select_first();
                    self.platform_desc_scroll_state = ScrollViewState::default();
                    self.spawn_readme_load_for_selected_package();
                }
                PlatformView::Templates => {
                    self.templates_list.select_first();
                    self.platform_desc_scroll_state = ScrollViewState::default();
                }
            },
            // End/G - go to last item
            KeyCode::End | KeyCode::Char('G') => match self.platform_view {
                PlatformView::Services => self.services_list.select_last(),
                PlatformView::Components => {
                    self.packages_list.select_last();
                    self.platform_desc_scroll_state = ScrollViewState::default();
                    self.spawn_readme_load_for_selected_package();
                }
                PlatformView::Templates => {
                    self.templates_list.select_last();
                    self.platform_desc_scroll_state = ScrollViewState::default();
                }
            },
            // Number keys - jump to specific view
            KeyCode::Char('1') => {
                self.platform_view = PlatformView::Services;
                self.platform_desc_scroll_state = ScrollViewState::default();
            }
            KeyCode::Char('2') => {
                self.platform_view = PlatformView::Components;
                self.platform_desc_scroll_state = ScrollViewState::default();
                self.spawn_readme_load_for_selected_package();
            }
            KeyCode::Char('3') => {
                self.platform_view = PlatformView::Templates;
                self.platform_desc_scroll_state = ScrollViewState::default();
            }
            _ => {}
        }
    }

    /// Spawn startup checks as background tasks (non-blocking)
    /// This allows the event loop to continue and the spinner to animate
    pub(super) fn spawn_startup_checks(&mut self) {
        self.startup_checks_started = true;

        // Set both checks to running state immediately
        self.startup_checks.token_check.status = CheckStatus::Running;
        self.startup_checks.cli_check.status = CheckStatus::Running;

        // Spawn token check (runs synchronously but in a blocking task)
        let tx = self.startup_result_tx.clone();
        tokio::spawn(async move {
            // Token check is synchronous so we wrap it
            let status = check_pulumi_token();
            let _ = tx
                .send(super::types::StartupCheckResult::TokenCheck(status))
                .await;
        });

        // Spawn CLI check (async)
        let tx = self.startup_result_tx.clone();
        tokio::spawn(async move {
            let status = check_pulumi_cli().await;
            let _ = tx
                .send(super::types::StartupCheckResult::CliCheck(status))
                .await;
        });
    }

    /// Process startup check results (non-blocking)
    pub(super) async fn process_startup_results(&mut self) {
        // Try to receive all pending results without blocking
        while let Ok(result) = self.startup_result_rx.try_recv() {
            match result {
                super::types::StartupCheckResult::TokenCheck(status) => {
                    self.startup_checks.token_check.status = status;
                }
                super::types::StartupCheckResult::CliCheck(status) => {
                    self.startup_checks.cli_check.status = status;
                }
            }
        }

        // If all checks completed and passed, load initial data
        if self.startup_checks.all_complete() && self.startup_checks.all_passed() {
            // Check if we haven't started loading data yet (only load once)
            if self.state.stacks.is_empty() && !self.is_loading && self.pending_data_loads == 0 {
                self.load_initial_data().await;
            }
        }
    }

    /// Handle splash screen key events
    pub(super) fn handle_splash_key(&mut self, key: KeyEvent) {
        // Check if startup checks are complete
        let checks_complete = self.startup_checks.all_complete();
        let checks_passed = self.startup_checks.all_passed();
        let checks_failed = self.startup_checks.any_failed();

        match key.code {
            // Space toggles the "don't show again" checkbox (only if checks passed)
            KeyCode::Char(' ') => {
                if checks_passed {
                    self.splash_dont_show_again = !self.splash_dont_show_again;
                }
            }
            // Enter dismisses the splash (only if checks passed)
            KeyCode::Enter => {
                if checks_complete && checks_passed {
                    self.dismiss_splash();
                }
            }
            // Escape also dismisses (only if checks passed)
            KeyCode::Esc => {
                if checks_complete && checks_passed {
                    self.dismiss_splash();
                }
            }
            // q quits the application (always available, especially when checks fail)
            KeyCode::Char('q') => {
                // Always allow quitting, but especially important when checks fail
                if checks_failed || checks_complete {
                    self.should_quit = true;
                }
            }
            _ => {}
        }
    }

    /// Dismiss the splash screen and save preferences
    pub(super) fn dismiss_splash(&mut self) {
        self.show_splash = false;

        // Save preference if "don't show again" is checked
        if self.splash_dont_show_again {
            self.config.show_splash = false;
            self.config.save();
        }
    }

    /// Handle Commands view keys
    async fn handle_commands_key(&mut self, key: KeyEvent) {
        match self.commands_view_state {
            CommandsViewState::BrowsingCategories => {
                self.handle_commands_categories_key(key);
            }
            CommandsViewState::BrowsingCommands => {
                self.handle_commands_list_key(key);
            }
            CommandsViewState::InputDialog => {
                self.handle_commands_input_key(key);
            }
            CommandsViewState::ConfirmDialog => {
                self.handle_commands_confirm_key(key);
            }
            CommandsViewState::OutputView => {
                self.handle_commands_output_key(key);
            }
        }
    }

    /// Handle keys when browsing categories
    fn handle_commands_categories_key(&mut self, key: KeyEvent) {
        if keys::is_up(&key) {
            self.commands_category_list.previous();
            self.update_commands_for_selected_category();
        } else if keys::is_down(&key) {
            self.commands_category_list.next();
            self.update_commands_for_selected_category();
        } else if keys::is_right(&key) || keys::is_enter(&key) {
            // Move focus to commands list
            self.commands_view_state = CommandsViewState::BrowsingCommands;
        } else if keys::is_char(&key, '/') {
            // Start filtering
            self.commands_is_filtering = true;
            self.commands_filter_input.set_focused(true);
        } else if keys::is_home(&key) || keys::is_char(&key, 'g') {
            self.commands_category_list.select_first();
            self.update_commands_for_selected_category();
        } else if keys::is_end(&key) || keys::is_char(&key, 'G') {
            self.commands_category_list.select_last();
            self.update_commands_for_selected_category();
        }
    }

    /// Handle keys when browsing commands
    fn handle_commands_list_key(&mut self, key: KeyEvent) {
        if keys::is_up(&key) {
            self.commands_command_list.previous();
        } else if keys::is_down(&key) {
            self.commands_command_list.next();
        } else if keys::is_left(&key) || keys::is_escape(&key) {
            // Move focus back to categories
            self.commands_view_state = CommandsViewState::BrowsingCategories;
        } else if keys::is_enter(&key) {
            // Execute the selected command
            self.start_command_execution();
        } else if keys::is_char(&key, '/') {
            // Start filtering
            self.commands_is_filtering = true;
            self.commands_filter_input.set_focused(true);
        } else if keys::is_home(&key) || keys::is_char(&key, 'g') {
            self.commands_command_list.select_first();
        } else if keys::is_end(&key) || keys::is_char(&key, 'G') {
            self.commands_command_list.select_last();
        } else {
            // Check for shortcut keys
            if let Some(c) = keys::get_char(&key) {
                if let Some(cmd) = self
                    .commands_command_list
                    .items()
                    .iter()
                    .find(|cmd| cmd.shortcut == Some(c))
                {
                    // Find and select the command
                    if let Some(idx) = self
                        .commands_command_list
                        .items()
                        .iter()
                        .position(|x| x.name == cmd.name)
                    {
                        self.commands_command_list.select(Some(idx));
                        self.start_command_execution();
                    }
                }
            }
        }
    }

    /// Handle keys in input dialog
    fn handle_commands_input_key(&mut self, key: KeyEvent) {
        if keys::is_escape(&key) {
            // Cancel and go back
            self.commands_view_state = CommandsViewState::BrowsingCommands;
            self.current_command_execution = None;
            self.commands_param_inputs.clear();
        } else if keys::is_enter(&key) {
            // Try to run the command
            self.finalize_command_params();
            if let Some(ref exec) = self.current_command_execution {
                if exec.command.needs_confirmation {
                    self.commands_view_state = CommandsViewState::ConfirmDialog;
                } else {
                    self.run_current_command();
                }
            }
        } else if keys::is_tab(&key) {
            // Move to next parameter
            if !self.commands_param_inputs.is_empty() {
                self.commands_param_inputs[self.commands_param_focus_index].set_focused(false);
                self.commands_param_focus_index =
                    (self.commands_param_focus_index + 1) % self.commands_param_inputs.len();
                self.commands_param_inputs[self.commands_param_focus_index].set_focused(true);
            }
        } else if keys::is_backtab(&key) {
            // Move to previous parameter
            if !self.commands_param_inputs.is_empty() {
                self.commands_param_inputs[self.commands_param_focus_index].set_focused(false);
                self.commands_param_focus_index = if self.commands_param_focus_index == 0 {
                    self.commands_param_inputs.len() - 1
                } else {
                    self.commands_param_focus_index - 1
                };
                self.commands_param_inputs[self.commands_param_focus_index].set_focused(true);
            }
        } else {
            // Pass to the focused input
            if let Some(input) = self
                .commands_param_inputs
                .get_mut(self.commands_param_focus_index)
            {
                input.handle_key(&key);
            }
        }
    }

    /// Handle keys in confirm dialog
    fn handle_commands_confirm_key(&mut self, key: KeyEvent) {
        if keys::is_char(&key, 'y') || keys::is_char(&key, 'Y') {
            // Confirmed, run the command
            self.run_current_command();
        } else if keys::is_char(&key, 'n') || keys::is_char(&key, 'N') || keys::is_escape(&key) {
            // Cancelled, go back to input
            self.commands_view_state = CommandsViewState::InputDialog;
        }
    }

    /// Handle keys in output view
    fn handle_commands_output_key(&mut self, key: KeyEvent) {
        if keys::is_escape(&key) {
            // Close output view and go back to commands
            self.commands_view_state = CommandsViewState::BrowsingCommands;
            self.current_command_execution = None;
            self.commands_output_scroll = ScrollViewState::default();
        } else if keys::is_char(&key, 'j') || keys::is_down(&key) {
            // Scroll down
            for _ in 0..3 {
                self.commands_output_scroll.scroll_down();
            }
        } else if keys::is_char(&key, 'k') || keys::is_up(&key) {
            // Scroll up
            for _ in 0..3 {
                self.commands_output_scroll.scroll_up();
            }
        } else if keys::is_page_down(&key) || keys::is_char(&key, 'J') {
            self.commands_output_scroll.scroll_page_down();
        } else if keys::is_page_up(&key) || keys::is_char(&key, 'K') {
            self.commands_output_scroll.scroll_page_up();
        } else if keys::is_char(&key, 'g') {
            self.commands_output_scroll.scroll_to_top();
        } else if keys::is_char(&key, 'G') {
            self.commands_output_scroll.scroll_to_bottom();
        }
    }

    /// Update commands list based on selected category
    fn update_commands_for_selected_category(&mut self) {
        if let Some(category) = self.commands_category_list.selected() {
            let commands = commands_by_category(*category);
            let has_commands = !commands.is_empty();
            self.commands_command_list.set_items(commands);
            if has_commands {
                self.commands_command_list.select(Some(0));
            }
        }
    }

    /// Start execution of the selected command
    fn start_command_execution(&mut self) {
        use crate::components::TextInput;

        if let Some(cmd) = self.commands_command_list.selected() {
            // Check if the command is interactive
            if cmd.execution_mode == ExecutionMode::Interactive {
                self.error = Some(format!(
                    "Command '{}' requires interactive mode.\nPlease run it directly in your terminal.",
                    cmd.name
                ));
                return;
            }

            // Create execution instance
            let execution = CommandExecution::new(cmd);

            // Create input fields for parameters
            self.commands_param_inputs = cmd
                .params
                .iter()
                .map(|param| {
                    let mut input = TextInput::new();
                    if let Some(default) = param.default {
                        input.set_value(default.to_string());
                    }
                    input
                })
                .collect();

            // Set focus to first parameter if any
            self.commands_param_focus_index = 0;
            if let Some(input) = self.commands_param_inputs.first_mut() {
                input.set_focused(true);
            }

            self.current_command_execution = Some(execution);

            // If no parameters, skip to confirmation or run
            if cmd.params.is_empty() {
                if cmd.needs_confirmation {
                    self.commands_view_state = CommandsViewState::ConfirmDialog;
                } else {
                    self.run_current_command();
                }
            } else {
                self.commands_view_state = CommandsViewState::InputDialog;
            }
        }
    }

    /// Finalize parameter values from inputs
    fn finalize_command_params(&mut self) {
        if let Some(ref mut exec) = self.current_command_execution {
            for (i, param) in exec.command.params.iter().enumerate() {
                if let Some(input) = self.commands_param_inputs.get(i) {
                    let value = input.value().to_string();
                    if !value.is_empty() {
                        exec.param_values.insert(param.name.to_string(), value);
                    }
                }
            }
        }
    }

    /// Run the current command
    fn run_current_command(&mut self) {
        if let Some(ref mut exec) = self.current_command_execution {
            // Validate the command
            if let Err(e) = can_run_command(exec) {
                self.error = Some(e);
                return;
            }

            // Update state to running
            exec.state = CommandExecutionState::Running;
            self.commands_view_state = CommandsViewState::OutputView;
            self.commands_output_scroll = ScrollViewState::default();

            // Spawn the command
            let tx = self.command_result_tx.clone();
            spawn_command(exec, tx);
        }
    }
}
