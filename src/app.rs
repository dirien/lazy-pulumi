//! Application state and main event loop
//!
//! This module contains the core application logic, state management,
//! and the main run loop.

use color_eyre::Result;
use crossterm::event::KeyEvent;
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::process::Command;
use tokio::sync::mpsc;
use tui_scrollview::ScrollViewState;

use crate::api::{
    EscEnvironmentSummary, NeoMessage, NeoMessageType, NeoTask, PulumiClient, Resource, Stack,
};
use crate::components::{Spinner, StatefulList, TextInput};
use crate::event::{keys, Event, EventHandler};
use crate::theme::Theme;
use crate::tui::{self, Tui};
use crate::ui;

/// NEO async operation result
#[derive(Debug)]
pub enum NeoAsyncResult {
    /// Task created successfully
    TaskCreated { task_id: String },
    /// Task events/messages received
    EventsReceived {
        messages: Vec<NeoMessage>,
        #[allow(dead_code)]
        has_more: bool,
    },
    /// Error occurred
    Error(String),
}

/// Application tabs/views
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Dashboard,
    Stacks,
    Esc,
    Neo,
}

impl Tab {
    pub fn all() -> &'static [Tab] {
        &[Tab::Dashboard, Tab::Stacks, Tab::Esc, Tab::Neo]
    }

    pub fn title(&self) -> &'static str {
        match self {
            Tab::Dashboard => " Dashboard ",
            Tab::Stacks => " Stacks ",
            Tab::Esc => " ESC ",
            Tab::Neo => " NEO ",
        }
    }

    pub fn index(&self) -> usize {
        match self {
            Tab::Dashboard => 0,
            Tab::Stacks => 1,
            Tab::Esc => 2,
            Tab::Neo => 3,
        }
    }

    pub fn from_index(index: usize) -> Self {
        match index {
            0 => Tab::Dashboard,
            1 => Tab::Stacks,
            2 => Tab::Esc,
            3 => Tab::Neo,
            _ => Tab::Dashboard,
        }
    }

    pub fn next(&self) -> Self {
        Tab::from_index((self.index() + 1) % Tab::all().len())
    }

    pub fn previous(&self) -> Self {
        let len = Tab::all().len();
        Tab::from_index((self.index() + len - 1) % len)
    }
}

/// Focus mode for input handling
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusMode {
    Normal,
    Input,
}

/// Application state
pub struct AppState {
    // Data
    pub stacks: Vec<Stack>,
    pub esc_environments: Vec<EscEnvironmentSummary>,
    pub neo_tasks: Vec<NeoTask>,
    pub resources: Vec<Resource>,

    // Selected stack details
    pub selected_stack_updates: Vec<(i32, String, String)>,

    // Selected ESC env details
    pub selected_env_yaml: Option<String>,
    pub selected_env_values: Option<serde_json::Value>,

    // NEO conversation
    pub neo_messages: Vec<NeoMessage>,
    pub current_task_id: Option<String>,

    // Organization
    pub organization: Option<String>,
    pub organizations: Vec<String>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            stacks: Vec::new(),
            esc_environments: Vec::new(),
            neo_tasks: Vec::new(),
            resources: Vec::new(),
            selected_stack_updates: Vec::new(),
            selected_env_yaml: None,
            selected_env_values: None,
            neo_messages: Vec::new(),
            current_task_id: None,
            organization: None,
            organizations: Vec::new(),
        }
    }
}

/// Main application
pub struct App {
    /// Terminal instance
    terminal: Tui,

    /// Event handler
    events: EventHandler,

    /// API client
    client: Option<PulumiClient>,

    /// Theme
    theme: Theme,

    /// Current tab
    tab: Tab,

    /// Focus mode
    focus: FocusMode,

    /// Show help popup
    show_help: bool,

    /// Show organization selector popup
    show_org_selector: bool,

    /// Organization list for selector
    org_list: StatefulList<String>,

    /// Loading state
    is_loading: bool,

    /// Loading spinner
    spinner: Spinner,

    /// Error message
    error: Option<String>,

    /// Should quit
    should_quit: bool,

    /// Application state
    pub state: AppState,

    // UI state
    stacks_list: StatefulList<Stack>,
    esc_list: StatefulList<EscEnvironmentSummary>,
    neo_tasks_list: StatefulList<NeoTask>,
    neo_input: TextInput,

    /// NEO polling state - tracks if we're waiting for agent response
    neo_polling: bool,
    /// Counter for polling interval (poll every N ticks)
    neo_poll_counter: u8,
    /// Counter for stable polls (no new messages for N consecutive polls)
    neo_stable_polls: u8,
    /// Previous message count (to detect changes)
    neo_prev_message_count: usize,
    /// Max polling attempts before giving up
    neo_max_polls: u8,
    /// Current poll count
    neo_current_poll: u8,
    /// Background poll counter for when NEO tab is active
    neo_bg_poll_counter: u8,
    /// NEO chat scroll view state
    neo_scroll_state: ScrollViewState,
    /// Auto-scroll to bottom when new messages arrive
    neo_auto_scroll: Arc<AtomicBool>,

    /// Channel for receiving async NEO results
    neo_result_rx: mpsc::Receiver<NeoAsyncResult>,
    /// Channel sender for NEO async tasks (wrapped in Arc for cloning)
    neo_result_tx: mpsc::Sender<NeoAsyncResult>,
}

impl App {
    /// Get the default organization from pulumi CLI
    async fn get_default_org() -> Option<String> {
        let output = Command::new("pulumi")
            .args(["org", "get-default"])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output()
            .await
            .ok()?;

        if output.status.success() {
            let org = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !org.is_empty() {
                return Some(org);
            }
        }
        None
    }

    /// Set the default organization using pulumi CLI
    /// Spawns in background fire-and-forget to avoid interfering with TUI
    fn spawn_set_default_org(org: String) {
        tokio::spawn(async move {
            let _ = Command::new("pulumi")
                .args(["org", "set-default", &org])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .output()
                .await;
        });
    }

    /// Create a new application
    pub async fn new() -> Result<Self> {
        let terminal = tui::init()?;
        let events = EventHandler::new(Duration::from_millis(100));
        let theme = Theme::new();

        // Try to create API client
        let client = match PulumiClient::new() {
            Ok(c) => Some(c),
            Err(e) => {
                tracing::warn!("Failed to create API client: {}", e);
                None
            }
        };

        // Create channel for async NEO results
        let (neo_result_tx, neo_result_rx) = mpsc::channel::<NeoAsyncResult>(32);

        let mut app = Self {
            terminal,
            events,
            client,
            theme,
            tab: Tab::Dashboard,
            focus: FocusMode::Normal,
            show_help: false,
            show_org_selector: false,
            org_list: StatefulList::new(),
            is_loading: false,
            spinner: Spinner::new(),
            error: None,
            should_quit: false,
            state: AppState::default(),
            stacks_list: StatefulList::new(),
            esc_list: StatefulList::new(),
            neo_tasks_list: StatefulList::new(),
            neo_input: TextInput::new(),
            neo_polling: false,
            neo_poll_counter: 0,
            neo_stable_polls: 0,
            neo_prev_message_count: 0,
            neo_max_polls: 60,  // Max 60 polls (~60 seconds at 1 poll/second)
            neo_current_poll: 0,
            neo_bg_poll_counter: 0,
            neo_scroll_state: ScrollViewState::default(),
            neo_auto_scroll: Arc::new(AtomicBool::new(true)),
            neo_result_rx,
            neo_result_tx,
        };

        // Initial data load
        app.load_initial_data().await;

        Ok(app)
    }

    /// Load initial data
    async fn load_initial_data(&mut self) {
        if let Some(ref client) = self.client {
            self.is_loading = true;
            self.spinner.set_message("Loading organizations...");

            // Get the default org from CLI first
            let default_org = Self::get_default_org().await;

            // Get organizations
            match client.list_organizations().await {
                Ok(orgs) => {
                    self.state.organizations = orgs.clone();
                    self.org_list.set_items(orgs);

                    // Use CLI default org if it exists in the list, otherwise fall back to first
                    let selected_org = default_org
                        .filter(|d| self.state.organizations.contains(d))
                        .or_else(|| self.state.organizations.first().cloned());

                    if let Some(org) = selected_org {
                        self.state.organization = Some(org);
                    }
                }
                Err(e) => {
                    self.error = Some(format!("Failed to load organizations: {}", e));
                }
            }

            // Load data for current org
            self.refresh_data().await;

            self.is_loading = false;
        } else {
            self.error = Some("No API client - set PULUMI_ACCESS_TOKEN".to_string());
        }
    }

    /// Refresh all data
    async fn refresh_data(&mut self) {
        if let Some(ref client) = self.client {
            let org = self.state.organization.as_deref();

            // Load stacks
            if let Ok(stacks) = client.list_stacks(org).await {
                self.state.stacks = stacks.clone();
                self.stacks_list.set_items(stacks);
            }

            // Load ESC environments
            if let Ok(envs) = client.list_esc_environments(org).await {
                self.state.esc_environments = envs.clone();
                self.esc_list.set_items(envs);
            }

            // Load NEO tasks
            if let Ok(tasks) = client.list_neo_tasks(org).await {
                self.state.neo_tasks = tasks.clone();
                self.neo_tasks_list.set_items(tasks);
            }

            // Load resources (sample search)
            if let Ok(resources) = client.search_resources(org, "").await {
                self.state.resources = resources;
            }
        }
    }

    /// Main run loop
    pub async fn run(&mut self) -> Result<()> {
        while !self.should_quit {
            // Render
            self.render()?;

            // Check for async NEO results (non-blocking)
            self.process_neo_results();

            // Handle events
            match self.events.next().await? {
                Event::Tick => {
                    self.spinner.tick();
                    // Poll for NEO updates if we're waiting for a response (fast polling)
                    if self.neo_polling {
                        self.neo_poll_counter += 1;
                        // Poll every 5 ticks (~500ms at 100ms tick rate)
                        if self.neo_poll_counter >= 5 {
                            self.neo_poll_counter = 0;
                            self.spawn_neo_poll();
                        }
                    }
                    // Background polling when NEO tab is active with a task selected
                    else if self.tab == Tab::Neo && self.state.current_task_id.is_some() {
                        self.neo_bg_poll_counter += 1;
                        // Background poll every 30 ticks (~3 seconds at 100ms tick rate)
                        if self.neo_bg_poll_counter >= 30 {
                            self.neo_bg_poll_counter = 0;
                            self.spawn_neo_poll();
                        }
                    }
                }
                Event::Key(key) => {
                    self.handle_key(key).await;
                }
                Event::Resize(_, _) => {
                    // Terminal will handle resize
                }
                Event::Mouse(_) => {
                    // Mouse handling (optional)
                }
                Event::Error(e) => {
                    self.error = Some(e);
                }
            }
        }

        // Cleanup
        tui::restore()?;

        Ok(())
    }

    /// Process any pending async NEO results
    fn process_neo_results(&mut self) {
        // Try to receive all pending results without blocking
        while let Ok(result) = self.neo_result_rx.try_recv() {
            match result {
                NeoAsyncResult::TaskCreated { task_id } => {
                    self.state.current_task_id = Some(task_id.clone());
                    // Add new task to list if not already there
                    if !self.state.neo_tasks.iter().any(|t| t.id == task_id) {
                        let msg_preview = self.state.neo_messages
                            .iter()
                            .find(|m| m.message_type == NeoMessageType::UserMessage)
                            .map(|m| {
                                let s: String = m.content.chars().take(50).collect();
                                if m.content.len() > 50 { format!("{}...", s) } else { s }
                            })
                            .unwrap_or_else(|| "New task".to_string());

                        let new_task = NeoTask {
                            id: task_id,
                            name: Some(msg_preview),
                            status: Some("running".to_string()),
                            created_at: Some(chrono::Utc::now().to_rfc3339()),
                            updated_at: None,
                            url: None,
                        };
                        self.state.neo_tasks.insert(0, new_task);
                        self.neo_tasks_list.set_items(self.state.neo_tasks.clone());
                        self.neo_tasks_list.select(Some(0));
                    }
                    // Start polling for updates
                    self.neo_polling = true;
                    self.neo_poll_counter = 5; // Trigger immediate poll on next tick
                }
                NeoAsyncResult::EventsReceived { messages, has_more: _ } => {
                    let current_count = messages.len();

                    // Only update if we got messages from the API
                    if !messages.is_empty() {
                        // Check if this is actually new content
                        let has_new_content = current_count != self.state.neo_messages.len()
                            || messages.iter().any(|m| {
                                !self.state.neo_messages.iter().any(|existing|
                                    existing.content == m.content && existing.message_type == m.message_type
                                )
                            });

                        if has_new_content {
                            self.state.neo_messages = messages;
                            // Auto-scroll to bottom if enabled
                            if self.neo_auto_scroll.load(Ordering::Relaxed) {
                                self.neo_scroll_state.scroll_to_bottom();
                            }
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

                    // Check for assistant response
                    let has_assistant_response = self.state.neo_messages
                        .iter()
                        .any(|m| m.message_type == NeoMessageType::AssistantMessage && !m.content.is_empty());

                    // Stop polling if:
                    // 1. We've had 10+ consecutive stable polls (no new messages for ~5 seconds)
                    //    AND we have at least one assistant message
                    // 2. OR we've hit max polls
                    let should_stop = (self.neo_stable_polls >= 10 && has_assistant_response)
                        || self.neo_current_poll >= self.neo_max_polls;

                    if should_stop {
                        self.neo_polling = false;
                        self.is_loading = false;
                        // Reset poll counters
                        self.neo_stable_polls = 0;
                        self.neo_prev_message_count = 0;
                        self.neo_current_poll = 0;
                    }
                }
                NeoAsyncResult::Error(e) => {
                    self.error = Some(format!("NEO error: {}", e));
                    self.neo_polling = false;
                    self.is_loading = false;
                    // Reset poll counters
                    self.neo_stable_polls = 0;
                    self.neo_prev_message_count = 0;
                    self.neo_current_poll = 0;
                }
            }
        }
    }

    /// Spawn async task to poll NEO events
    fn spawn_neo_poll(&self) {
        if let (Some(task_id), Some(org)) = (&self.state.current_task_id, &self.state.organization) {
            if let Some(ref client) = self.client {
                let client = client.clone();
                let task_id = task_id.clone();
                let org = org.clone();
                let tx = self.neo_result_tx.clone();

                tokio::spawn(async move {
                    match client.get_neo_task_events(&org, &task_id).await {
                        Ok(response) => {
                            let _ = tx.send(NeoAsyncResult::EventsReceived {
                                messages: response.messages,
                                has_more: response.has_more,
                            }).await;
                        }
                        Err(e) => {
                            tracing::warn!("Failed to poll NEO task: {}", e);
                            // Don't send error for transient poll failures
                        }
                    }
                });
            }
        }
    }

    /// Poll for NEO task updates
    #[allow(dead_code)]
    async fn poll_neo_task(&mut self) {
        if let Some(task_id) = &self.state.current_task_id.clone() {
            if let Some(ref client) = self.client {
                if let Some(org) = &self.state.organization.clone() {
                    match client.get_neo_task_events(org, task_id).await {
                        Ok(response) => {
                            // Update messages if we got new ones
                            if !response.messages.is_empty() {
                                self.state.neo_messages = response.messages;
                            }
                            // Check if task is still running by looking at last message
                            // If we have an assistant response and no has_more, stop polling
                            if !response.has_more {
                                // Check if we have a substantive response
                                let has_assistant_response = self.state.neo_messages
                                    .iter()
                                    .any(|m| m.role == "assistant" && !m.content.is_empty());
                                if has_assistant_response {
                                    self.neo_polling = false;
                                    self.is_loading = false;
                                }
                            }
                        }
                        Err(e) => {
                            tracing::warn!("Failed to poll NEO task: {}", e);
                            // Don't stop polling on transient errors
                        }
                    }
                }
            }
        }
    }

    /// Render the UI
    fn render(&mut self) -> Result<()> {
        // Extract values before the closure to avoid borrow issues
        let theme = &self.theme;
        let tab = self.tab;
        let org = self.state.organization.as_deref();
        let show_help = self.show_help;
        let show_org_selector = self.show_org_selector;
        let is_loading = self.is_loading;
        // For NEO tab, show spinner when polling (waiting for response)
        let neo_is_thinking = self.neo_polling || self.is_loading;
        let spinner_char = self.spinner.char();
        let spinner_message = self.spinner.message();
        let error_msg = self.error.clone();

        // Get the footer hint before the closure
        let hint = self.get_footer_hint();

        // References to state
        let state = &self.state;
        let stacks_list = &mut self.stacks_list;
        let esc_list = &mut self.esc_list;
        let neo_tasks_list = &mut self.neo_tasks_list;
        let neo_input = &self.neo_input;
        let org_list = &mut self.org_list;
        let neo_scroll_state = &mut self.neo_scroll_state;
        let neo_auto_scroll = self.neo_auto_scroll.clone();

        self.terminal.draw(|frame| {
            let (header_area, content_area, footer_area) = ui::main_layout(frame.area());

            // Header with tabs
            ui::render_header(frame, theme, header_area, tab, org);

            // Content based on current tab
            match tab {
                Tab::Dashboard => {
                    ui::render_dashboard(frame, theme, content_area, state);
                }
                Tab::Stacks => {
                    ui::render_stacks_view(
                        frame,
                        theme,
                        content_area,
                        stacks_list,
                        &state.selected_stack_updates,
                    );
                }
                Tab::Esc => {
                    ui::render_esc_view(
                        frame,
                        theme,
                        content_area,
                        esc_list,
                        state.selected_env_yaml.as_deref(),
                        state.selected_env_values.as_ref(),
                    );
                }
                Tab::Neo => {
                    ui::render_neo_view(
                        frame,
                        theme,
                        content_area,
                        neo_tasks_list,
                        &state.neo_messages,
                        neo_input,
                        neo_scroll_state,
                        &neo_auto_scroll,
                        neo_is_thinking,
                        spinner_char,
                    );
                }
            }

            // Footer
            ui::render_footer(frame, theme, footer_area, &hint);

            // Organization selector popup
            if show_org_selector {
                ui::render_org_selector(frame, theme, org_list, org);
            }

            // Help popup
            if show_help {
                ui::render_help(frame, theme);
            }

            // Error popup
            if let Some(ref error) = error_msg {
                ui::render_error_popup(frame, theme, error);
            }

            // Loading overlay
            if is_loading && tab != Tab::Neo {
                ui::render_loading(frame, theme, spinner_message, spinner_char);
            }
        })?;

        Ok(())
    }

    /// Get contextual footer hint
    fn get_footer_hint(&self) -> String {
        if self.show_help {
            return "Press ? or Esc to close help".to_string();
        }

        if self.show_org_selector {
            return "↑↓: navigate | Enter: select | Esc: cancel".to_string();
        }

        if self.error.is_some() {
            return "Press Esc to dismiss error".to_string();
        }

        match self.focus {
            FocusMode::Input => "Enter: send | Esc: cancel".to_string(),
            FocusMode::Normal => match self.tab {
                Tab::Dashboard => "Tab: switch | o: org | ?: help | r: refresh | q: quit".to_string(),
                Tab::Stacks => "↑↓: navigate | o: org | Enter: details | r: refresh | q: quit".to_string(),
                Tab::Esc => "↑↓: navigate | o: org | Enter: load | O: resolve | q: quit".to_string(),
                Tab::Neo => "↑↓: tasks | j/k: scroll | o: org | n: new | i: type | q: quit".to_string(),
            },
        }
    }

    /// Handle key events
    async fn handle_key(&mut self, key: KeyEvent) {
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

        // Handle organization selector popup
        if self.show_org_selector {
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

                    // Refresh all data for the new organization
                    self.refresh_data().await;
                    self.is_loading = false;
                }
            }
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
            self.tab = self.tab.next();
            return;
        }

        if keys::is_backtab(&key) {
            self.tab = self.tab.previous();
            return;
        }

        if keys::is_char(&key, 'r') {
            self.is_loading = true;
            self.spinner.set_message("Refreshing...");
            self.refresh_data().await;
            self.is_loading = false;
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
                                let time = u.start_time.map(|t| {
                                    chrono::DateTime::from_timestamp(t, 0)
                                        .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
                                        .unwrap_or_else(|| "Unknown".to_string())
                                }).unwrap_or_else(|| "Unknown".to_string());

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

    /// Handle NEO view keys
    async fn handle_neo_key(&mut self, key: KeyEvent) {
        if keys::is_char(&key, 'i') {
            self.focus = FocusMode::Input;
            self.neo_input.set_focused(true);
        } else if keys::is_char(&key, 'n') {
            // Start new task
            self.state.neo_messages.clear();
            self.state.current_task_id = None;
            self.neo_scroll_state = ScrollViewState::default();
            self.neo_auto_scroll.store(true, Ordering::Relaxed);
            self.focus = FocusMode::Input;
            self.neo_input.set_focused(true);
        } else if keys::is_up(&key) {
            // Navigate task list
            self.neo_tasks_list.previous();
        } else if keys::is_down(&key) {
            // Navigate task list
            self.neo_tasks_list.next();
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
            self.neo_scroll_state.scroll_to_bottom();
            self.neo_auto_scroll.store(true, Ordering::Relaxed);
        } else if keys::is_char(&key, 'g') {
            // Scroll to top (oldest messages)
            self.neo_scroll_state.scroll_to_top();
            self.neo_auto_scroll.store(false, Ordering::Relaxed);
        } else if keys::is_enter(&key) {
            // Only load task events when Enter is pressed
            self.load_selected_task().await;
        }
    }

    /// Load selected NEO task
    async fn load_selected_task(&mut self) {
        if let Some(task) = self.neo_tasks_list.selected() {
            self.state.current_task_id = Some(task.id.clone());
            self.state.neo_messages.clear();
            self.neo_scroll_state = ScrollViewState::default();
            self.neo_auto_scroll.store(true, Ordering::Relaxed);
            // Reset background poll counter to start fresh polling cycle
            self.neo_bg_poll_counter = 0;

            // Try to continue/poll the task to get messages
            if let Some(ref client) = self.client {
                if let Some(org) = &self.state.organization {
                    self.is_loading = true;

                    if let Ok(response) = client.continue_neo_task(org, &task.id, None).await {
                        self.state.neo_messages = response.messages;
                        // Scroll to bottom after loading messages
                        self.neo_scroll_state.scroll_to_bottom();
                    }

                    self.is_loading = false;
                }
            }
        }
    }

    /// Send a message to NEO (non-blocking)
    fn send_neo_message(&mut self) {
        let message = self.neo_input.take();
        if message.trim().is_empty() {
            return;
        }

        // Add user message to chat immediately
        self.state.neo_messages.push(NeoMessage {
            role: "user".to_string(),
            content: message.clone(),
            message_type: NeoMessageType::UserMessage,
            timestamp: None,
            tool_calls: vec![],
            tool_name: None,
        });

        self.focus = FocusMode::Normal;
        self.neo_input.set_focused(false);
        self.is_loading = true;
        self.spinner.set_message("NEO is thinking...");

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
                        client.continue_neo_task(&org, &tid, Some(&message)).await
                    } else {
                        // Create new task
                        client.create_neo_task(&org, &message).await
                    };

                    match result {
                        Ok(response) => {
                            // Send task created result
                            let _ = tx.send(NeoAsyncResult::TaskCreated {
                                task_id: response.task_id,
                            }).await;
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
                // Scroll to bottom and enable auto-scroll
                self.neo_scroll_state.scroll_to_bottom();
                self.neo_auto_scroll.store(true, Ordering::Relaxed);
            }
        }
    }
}
