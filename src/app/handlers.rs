//! Key event handlers
//!
//! This module contains all keyboard event handling logic for the application,
//! organized by context (global, tab-specific, popup-specific).

use crossterm::event::{KeyCode, KeyEvent};
use std::sync::atomic::Ordering;
use tui_scrollview::ScrollViewState;

use crate::event::keys;
use crate::logging;
use crate::startup::{check_pulumi_cli, check_pulumi_token, CheckStatus};

use super::types::{FocusMode, PlatformView, Tab};
use super::App;

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

        // Handle input mode
        if self.focus == FocusMode::Input {
            if keys::is_escape(&key) {
                self.focus = FocusMode::Normal;
                self.neo_input.set_focused(false);
            } else if keys::is_enter(&key) {
                self.send_neo_message();
            } else {
                self.neo_input.handle_key(&key);
            }
            return;
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
            self.logs_cache = logging::read_log_tail(None);
            // Auto-scroll to bottom
            let total_lines = self.logs_cache.len();
            self.logs_scroll_offset = total_lines.saturating_sub(20);
            self.show_logs = true;
            return;
        }

        // Open organization selector with 'o'
        if keys::is_char(&key, 'o') {
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
            if self.tab == Tab::Neo && old_tab != Tab::Neo {
                if self.state.current_task_id.is_none() {
                    self.neo_hide_task_list = false;
                }
            }
            return;
        }

        if keys::is_backtab(&key) {
            let old_tab = self.tab;
            self.tab = self.tab.previous();
            // When switching to Neo tab, show task list unless there's an active task
            if self.tab == Tab::Neo && old_tab != Tab::Neo {
                if self.state.current_task_id.is_none() {
                    self.neo_hide_task_list = false;
                }
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
        }
    }

    /// Handle logs popup keys
    fn handle_logs_key(&mut self, key: KeyEvent) {
        if keys::is_escape(&key) || keys::is_char(&key, 'l') {
            self.show_logs = false;
        } else if keys::is_char(&key, 'w') {
            // Toggle word wrap
            self.logs_word_wrap = !self.logs_word_wrap;
            // Reset scroll position when toggling wrap mode
            self.logs_scroll_offset = 0;
        } else if keys::is_char(&key, 'j') || keys::is_down(&key) {
            // Scroll down
            self.logs_scroll_offset = self.logs_scroll_offset.saturating_add(3);
        } else if keys::is_char(&key, 'k') || keys::is_up(&key) {
            // Scroll up
            self.logs_scroll_offset = self.logs_scroll_offset.saturating_sub(3);
        } else if keys::is_char(&key, 'g') {
            // Jump to top
            self.logs_scroll_offset = 0;
        } else if keys::is_char(&key, 'G') {
            // Jump to bottom
            let total_lines = self.logs_cache.len();
            self.logs_scroll_offset = total_lines.saturating_sub(20);
        } else if keys::is_page_down(&key) || keys::is_char(&key, 'J') {
            self.logs_scroll_offset = self.logs_scroll_offset.saturating_add(20);
        } else if keys::is_page_up(&key) || keys::is_char(&key, 'K') {
            self.logs_scroll_offset = self.logs_scroll_offset.saturating_sub(20);
        } else if keys::is_char(&key, 'R') {
            // Refresh logs
            self.logs_cache = logging::read_log_tail(None);
            // Auto-scroll to bottom on refresh
            let total_lines = self.logs_cache.len();
            self.logs_scroll_offset = total_lines.saturating_sub(20);
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
        if keys::is_up(&key) {
            self.esc_list.previous();
            self.state.selected_env_yaml = None;
            self.state.selected_env_values = None;
        } else if keys::is_down(&key) {
            self.esc_list.next();
            self.state.selected_env_yaml = None;
            self.state.selected_env_values = None;
        } else if keys::is_home(&key) || keys::is_char(&key, 'g') {
            self.esc_list.select_first();
        } else if keys::is_end(&key) || keys::is_char(&key, 'G') {
            self.esc_list.select_last();
        } else if keys::is_enter(&key) {
            // Load environment definition
            if let Some(env) = self.esc_list.selected() {
                if let Some(ref client) = self.client {
                    self.is_loading = true;
                    self.spinner.set_message("Loading definition...");

                    if let Ok(details) = client
                        .get_esc_environment(&env.organization, &env.project, &env.name)
                        .await
                    {
                        self.state.selected_env_yaml = details.yaml;
                    }

                    self.is_loading = false;
                }
            }
        } else if keys::is_char(&key, 'O') {
            // Open and resolve environment
            if let Some(env) = self.esc_list.selected() {
                if let Some(ref client) = self.client {
                    self.is_loading = true;
                    self.spinner.set_message("Opening environment...");

                    if let Ok(response) = client
                        .open_esc_environment(&env.organization, &env.project, &env.name)
                        .await
                    {
                        self.state.selected_env_values = response.values;
                    }

                    self.is_loading = false;
                }
            }
        }
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

    /// Run startup checks asynchronously
    pub(super) async fn run_startup_checks(&mut self) {
        self.startup_checks_started = true;

        // Run token check first (synchronous)
        self.startup_checks.token_check.status = CheckStatus::Running;
        // Render to show running state
        let _ = self.render();
        self.startup_checks.token_check.status = check_pulumi_token();

        // Run CLI check (async)
        self.startup_checks.cli_check.status = CheckStatus::Running;
        // Render to show running state
        let _ = self.render();
        self.startup_checks.cli_check.status = check_pulumi_cli().await;

        // If all checks passed, load initial data
        if self.startup_checks.all_passed() {
            self.load_initial_data().await;
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
}
