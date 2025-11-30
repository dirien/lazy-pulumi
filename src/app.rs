//! Application state and main event loop
//!
//! This module contains the core application logic, state management,
//! and the main run loop.

use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEvent};
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::process::Command;
use tokio::sync::mpsc;
use tui_scrollview::ScrollViewState;

use crate::config::Config;

use crate::api::{
    EscEnvironmentSummary, NeoMessage, NeoMessageType, NeoTask, PulumiClient, RegistryPackage,
    RegistryTemplate, Resource, Service, Stack,
};
use crate::components::{Spinner, StatefulList, TextInput};
use crate::event::{keys, Event, EventHandler};
use crate::logging;
use crate::theme::Theme;
use crate::tui::{self, Tui};
use crate::ui;

/// Async data loading result
#[derive(Debug)]
pub enum DataLoadResult {
    Stacks(Vec<Stack>),
    EscEnvironments(Vec<EscEnvironmentSummary>),
    NeoTasks(Vec<NeoTask>),
    Resources(Vec<Resource>),
    Services(Vec<Service>),
    RegistryPackages(Vec<RegistryPackage>),
    RegistryTemplates(Vec<RegistryTemplate>),
    /// README content loaded for a package (key, content)
    ReadmeContent { package_key: String, content: String },
    Error(String),
}

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
    Platform,
}

impl Tab {
    pub fn all() -> &'static [Tab] {
        &[Tab::Dashboard, Tab::Stacks, Tab::Esc, Tab::Neo, Tab::Platform]
    }

    pub fn title(&self) -> &'static str {
        match self {
            Tab::Dashboard => " Dashboard ",
            Tab::Stacks => " Stacks ",
            Tab::Esc => " Environment ",
            Tab::Neo => " NEO ",
            Tab::Platform => " Platform ",
        }
    }

    pub fn index(&self) -> usize {
        match self {
            Tab::Dashboard => 0,
            Tab::Stacks => 1,
            Tab::Esc => 2,
            Tab::Neo => 3,
            Tab::Platform => 4,
        }
    }

    pub fn from_index(index: usize) -> Self {
        match index {
            0 => Tab::Dashboard,
            1 => Tab::Stacks,
            2 => Tab::Esc,
            3 => Tab::Neo,
            4 => Tab::Platform,
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

/// Platform sub-view selection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlatformView {
    Services,
    Components,
    Templates,
}

impl PlatformView {
    pub fn all() -> &'static [PlatformView] {
        &[PlatformView::Services, PlatformView::Components, PlatformView::Templates]
    }

    pub fn title(&self) -> &'static str {
        match self {
            PlatformView::Services => "Services",
            PlatformView::Components => "Components",
            PlatformView::Templates => "Templates",
        }
    }

    pub fn index(&self) -> usize {
        match self {
            PlatformView::Services => 0,
            PlatformView::Components => 1,
            PlatformView::Templates => 2,
        }
    }

    pub fn from_index(index: usize) -> Self {
        match index {
            0 => PlatformView::Services,
            1 => PlatformView::Components,
            2 => PlatformView::Templates,
            _ => PlatformView::Services,
        }
    }

    pub fn next(&self) -> Self {
        PlatformView::from_index((self.index() + 1) % PlatformView::all().len())
    }

    pub fn previous(&self) -> Self {
        let len = PlatformView::all().len();
        PlatformView::from_index((self.index() + len - 1) % len)
    }
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

    // Platform data
    pub services: Vec<Service>,
    pub registry_packages: Vec<RegistryPackage>,
    pub registry_templates: Vec<RegistryTemplate>,

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
            services: Vec::new(),
            registry_packages: Vec::new(),
            registry_templates: Vec::new(),
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

    /// Show splash screen on startup
    show_splash: bool,

    /// Splash screen start time (for minimum display duration)
    splash_start_time: Option<Instant>,

    /// Minimum splash screen duration in seconds
    splash_min_duration: Duration,

    /// Whether the "don't show again" checkbox is selected
    splash_dont_show_again: bool,

    /// User configuration
    config: Config,

    /// Show help popup
    show_help: bool,

    /// Show organization selector popup
    show_org_selector: bool,

    /// Show logs popup
    show_logs: bool,

    /// Log viewer scroll offset
    logs_scroll_offset: usize,

    /// Log viewer word wrap enabled
    logs_word_wrap: bool,

    /// Cached log lines
    logs_cache: Vec<String>,

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

    // Platform UI state
    platform_view: PlatformView,
    services_list: StatefulList<Service>,
    packages_list: StatefulList<RegistryPackage>,
    templates_list: StatefulList<RegistryTemplate>,
    /// Scroll state for Component/Template description panel
    platform_desc_scroll_state: ScrollViewState,

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

    /// Channel for receiving async data loading results
    data_result_rx: mpsc::Receiver<DataLoadResult>,
    /// Channel sender for async data loading
    data_result_tx: mpsc::Sender<DataLoadResult>,
    /// Number of pending data load operations
    pending_data_loads: u8,
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

        // Load user configuration
        let config = Config::load();

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

        // Create channel for async data loading results
        let (data_result_tx, data_result_rx) = mpsc::channel::<DataLoadResult>(32);

        // Determine if splash should be shown based on config
        let show_splash = config.show_splash;

        let mut app = Self {
            terminal,
            events,
            client,
            theme,
            tab: Tab::Dashboard,
            focus: FocusMode::Normal,
            show_splash,
            splash_start_time: if show_splash { Some(Instant::now()) } else { None },
            splash_min_duration: Duration::from_secs(5),
            splash_dont_show_again: false,
            config,
            show_help: false,
            show_org_selector: false,
            show_logs: false,
            logs_scroll_offset: 0,
            logs_word_wrap: false,
            logs_cache: Vec::new(),
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
            platform_view: PlatformView::Services,
            services_list: StatefulList::new(),
            packages_list: StatefulList::new(),
            templates_list: StatefulList::new(),
            platform_desc_scroll_state: ScrollViewState::default(),
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
            data_result_rx,
            data_result_tx,
            pending_data_loads: 0,
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

            // Load data for current org (non-blocking)
            self.refresh_data();
            // Note: is_loading will be cleared when all spawned tasks complete
        } else {
            self.error = Some("No API client - set PULUMI_ACCESS_TOKEN".to_string());
        }
    }

    /// Refresh all data - spawns parallel async tasks for non-blocking loads
    fn refresh_data(&mut self) {
        if let Some(ref client) = self.client {
            let org = self.state.organization.clone();
            let tx = self.data_result_tx.clone();

            // Track how many loads we're starting
            self.pending_data_loads = 7;
            self.is_loading = true;
            self.spinner.set_message("Loading data...");

            // Spawn all data loads in parallel
            let client1 = client.clone();
            let org1 = org.clone();
            let tx1 = tx.clone();
            tokio::spawn(async move {
                match client1.list_stacks(org1.as_deref()).await {
                    Ok(stacks) => { let _ = tx1.send(DataLoadResult::Stacks(stacks)).await; }
                    Err(e) => { let _ = tx1.send(DataLoadResult::Error(format!("Stacks: {}", e))).await; }
                }
            });

            let client2 = client.clone();
            let org2 = org.clone();
            let tx2 = tx.clone();
            tokio::spawn(async move {
                match client2.list_esc_environments(org2.as_deref()).await {
                    Ok(envs) => { let _ = tx2.send(DataLoadResult::EscEnvironments(envs)).await; }
                    Err(e) => { let _ = tx2.send(DataLoadResult::Error(format!("ESC: {}", e))).await; }
                }
            });

            let client3 = client.clone();
            let org3 = org.clone();
            let tx3 = tx.clone();
            tokio::spawn(async move {
                match client3.list_neo_tasks(org3.as_deref()).await {
                    Ok(tasks) => { let _ = tx3.send(DataLoadResult::NeoTasks(tasks)).await; }
                    Err(e) => { let _ = tx3.send(DataLoadResult::Error(format!("NEO: {}", e))).await; }
                }
            });

            let client4 = client.clone();
            let org4 = org.clone();
            let tx4 = tx.clone();
            tokio::spawn(async move {
                match client4.search_resources(org4.as_deref(), "").await {
                    Ok(resources) => { let _ = tx4.send(DataLoadResult::Resources(resources)).await; }
                    Err(e) => { let _ = tx4.send(DataLoadResult::Error(format!("Resources: {}", e))).await; }
                }
            });

            let client5 = client.clone();
            let org5 = org.clone();
            let tx5 = tx.clone();
            tokio::spawn(async move {
                match client5.list_services(org5.as_deref()).await {
                    Ok(services) => { let _ = tx5.send(DataLoadResult::Services(services)).await; }
                    Err(e) => { let _ = tx5.send(DataLoadResult::Error(format!("Services: {}", e))).await; }
                }
            });

            let client6 = client.clone();
            let org6 = org.clone();
            let tx6 = tx.clone();
            tokio::spawn(async move {
                match client6.list_registry_packages(org6.as_deref()).await {
                    Ok(packages) => { let _ = tx6.send(DataLoadResult::RegistryPackages(packages)).await; }
                    Err(e) => { let _ = tx6.send(DataLoadResult::Error(format!("Packages: {}", e))).await; }
                }
            });

            let client7 = client.clone();
            let org7 = org;
            let tx7 = tx;
            tokio::spawn(async move {
                match client7.list_registry_templates(org7.as_deref()).await {
                    Ok(templates) => { let _ = tx7.send(DataLoadResult::RegistryTemplates(templates)).await; }
                    Err(e) => { let _ = tx7.send(DataLoadResult::Error(format!("Templates: {}", e))).await; }
                }
            });
        }
    }

    /// Process async data loading results (non-blocking)
    fn process_data_results(&mut self) {
        while let Ok(result) = self.data_result_rx.try_recv() {
            self.pending_data_loads = self.pending_data_loads.saturating_sub(1);

            match result {
                DataLoadResult::Stacks(stacks) => {
                    self.state.stacks = stacks.clone();
                    self.stacks_list.set_items(stacks);
                }
                DataLoadResult::EscEnvironments(envs) => {
                    self.state.esc_environments = envs.clone();
                    self.esc_list.set_items(envs);
                }
                DataLoadResult::NeoTasks(tasks) => {
                    self.state.neo_tasks = tasks.clone();
                    self.neo_tasks_list.set_items(tasks);
                }
                DataLoadResult::Resources(resources) => {
                    self.state.resources = resources;
                }
                DataLoadResult::Services(services) => {
                    self.state.services = services.clone();
                    self.services_list.set_items(services);
                }
                DataLoadResult::RegistryPackages(packages) => {
                    self.state.registry_packages = packages.clone();
                    self.packages_list.set_items(packages);
                }
                DataLoadResult::RegistryTemplates(templates) => {
                    self.state.registry_templates = templates.clone();
                    self.templates_list.set_items(templates);
                }
                DataLoadResult::ReadmeContent { package_key, content } => {
                    // Find the package and update its readme_content
                    if let Some(pkg) = self.packages_list.items_mut().iter_mut()
                        .find(|p| p.key() == package_key)
                    {
                        pkg.readme_content = Some(content);
                    }
                }
                DataLoadResult::Error(e) => {
                    tracing::warn!("Data load error: {}", e);
                }
            }

            // Clear loading state when all loads complete
            if self.pending_data_loads == 0 {
                self.is_loading = false;
                // Note: splash screen is now dismissed via user interaction, not auto-hide
            }
        }
    }

    /// Main run loop
    pub async fn run(&mut self) -> Result<()> {
        while !self.should_quit {
            // Render
            self.render()?;

            // Check for async data loading results (non-blocking)
            self.process_data_results();

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
        let show_splash = self.show_splash;
        let splash_dont_show_again = self.splash_dont_show_again;
        let show_help = self.show_help;
        let show_org_selector = self.show_org_selector;
        let show_logs = self.show_logs;
        let logs_scroll_offset = self.logs_scroll_offset;
        let logs_word_wrap = self.logs_word_wrap;
        let logs_cache = &self.logs_cache;
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

        // Platform state
        let platform_view = self.platform_view;
        let services_list = &mut self.services_list;
        let packages_list = &mut self.packages_list;
        let templates_list = &mut self.templates_list;
        let platform_desc_scroll_state = &mut self.platform_desc_scroll_state;

        self.terminal.draw(|frame| {
            // Show splash screen (minimum 5 seconds or until dismissed)
            if show_splash {
                ui::render_splash(frame, theme, spinner_char, splash_dont_show_again, is_loading);
                return;
            }

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
                Tab::Platform => {
                    ui::render_platform_view(
                        frame,
                        theme,
                        content_area,
                        platform_view,
                        services_list,
                        packages_list,
                        templates_list,
                        platform_desc_scroll_state,
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

            // Logs popup
            if show_logs {
                ui::render_logs(frame, theme, logs_cache, logs_scroll_offset, logs_word_wrap);
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

        if self.show_logs {
            return "j/k: scroll | g/G: top/bottom | w: wrap | R: refresh | l/Esc: close".to_string();
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
                Tab::Dashboard => "Tab: switch | o: org | l: logs | ?: help | r: refresh | q: quit".to_string(),
                Tab::Stacks => "↑↓: navigate | o: org | l: logs | Enter: details | r: refresh | q: quit".to_string(),
                Tab::Esc => "↑↓: navigate | o: org | l: logs | Enter: load | O: resolve | q: quit".to_string(),
                Tab::Neo => "↑↓: tasks | j/k: scroll | o: org | l: logs | n: new | i: type | q: quit".to_string(),
                Tab::Platform => "↑↓: navigate | ←→: switch view | o: org | l: logs | r: refresh | q: quit".to_string(),
            },
        }
    }

    /// Handle key events
    async fn handle_key(&mut self, key: KeyEvent) {
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

        // Handle logs popup
        if self.show_logs {
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

                    // Refresh all data for the new organization (non-blocking)
                    self.refresh_data();
                    // Note: is_loading will be cleared when all spawned tasks complete
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
            self.tab = self.tab.next();
            return;
        }

        if keys::is_backtab(&key) {
            self.tab = self.tab.previous();
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

    /// Load README for the currently selected package (if not already loaded)
    fn spawn_readme_load_for_selected_package(&self) {
        let Some(client) = &self.client else {
            return;
        };
        if let Some(pkg) = self.packages_list.selected() {
            // Only load if README URL exists and content hasn't been loaded yet
            if pkg.readme_content.is_some() {
                return;
            }
            if let Some(readme_url) = &pkg.readme_url {
                let client = client.clone();
                let tx = self.data_result_tx.clone();
                let package_key = pkg.key();
                let url = readme_url.clone();

                tokio::spawn(async move {
                    match client.fetch_readme(&url).await {
                        Ok(content) => {
                            let _ = tx.send(DataLoadResult::ReadmeContent {
                                package_key,
                                content,
                            }).await;
                        }
                        Err(e) => {
                            tracing::debug!("Failed to load README: {}", e);
                        }
                    }
                });
            }
        }
    }

    /// Handle Platform view keys
    async fn handle_platform_key(&mut self, key: KeyEvent) {
        use crossterm::event::KeyCode;

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

    /// Handle splash screen key events
    fn handle_splash_key(&mut self, key: KeyEvent) {
        // Check if minimum time has passed
        let min_time_passed = self.splash_start_time
            .map(|start| start.elapsed() >= self.splash_min_duration)
            .unwrap_or(true);

        match key.code {
            // Space toggles the "don't show again" checkbox
            KeyCode::Char(' ') => {
                self.splash_dont_show_again = !self.splash_dont_show_again;
            }
            // Enter dismisses the splash (if min time passed and not loading)
            KeyCode::Enter => {
                if min_time_passed && !self.is_loading {
                    self.dismiss_splash();
                }
            }
            // Escape also dismisses (if min time passed and not loading)
            KeyCode::Esc => {
                if min_time_passed && !self.is_loading {
                    self.dismiss_splash();
                }
            }
            // q quits the application
            KeyCode::Char('q') => {
                self.should_quit = true;
            }
            _ => {}
        }
    }

    /// Dismiss the splash screen and save preferences
    fn dismiss_splash(&mut self) {
        self.show_splash = false;

        // Save preference if "don't show again" is checked
        if self.splash_dont_show_again {
            self.config.show_splash = false;
            self.config.save();
        }
    }
}
