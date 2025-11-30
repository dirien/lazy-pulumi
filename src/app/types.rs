//! Application types and state definitions
//!
//! This module contains the core type definitions used throughout the application,
//! including enums for tabs, focus modes, and the main application state struct.

use crate::api::{
    EscEnvironmentSummary, NeoMessage, NeoTask, RegistryPackage, RegistryTemplate, Resource,
    Service, Stack,
};

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

/// Neo async operation result
#[derive(Debug)]
pub enum NeoAsyncResult {
    /// Task created successfully
    TaskCreated { task_id: String },
    /// Task events/messages received
    EventsReceived {
        messages: Vec<NeoMessage>,
        #[allow(dead_code)]
        has_more: bool,
        /// Task status from API (e.g., "running", "idle", "completed")
        /// Used to determine if we should keep polling/showing thinking indicator
        task_status: Option<String>,
    },
    /// Error occurred
    Error(String),
}

/// Startup check async result
#[derive(Debug)]
pub enum StartupCheckResult {
    /// Token check completed
    TokenCheck(crate::startup::CheckStatus),
    /// CLI check completed
    CliCheck(crate::startup::CheckStatus),
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
        // Neo is second after Dashboard
        &[Tab::Dashboard, Tab::Neo, Tab::Stacks, Tab::Esc, Tab::Platform]
    }

    pub fn title(&self) -> &'static str {
        match self {
            Tab::Dashboard => " Dashboard ",
            Tab::Stacks => " Stacks ",
            Tab::Esc => " Environment ",
            Tab::Neo => " Neo ",
            Tab::Platform => " Platform ",
        }
    }

    pub fn index(&self) -> usize {
        match self {
            Tab::Dashboard => 0,
            Tab::Neo => 1,
            Tab::Stacks => 2,
            Tab::Esc => 3,
            Tab::Platform => 4,
        }
    }

    pub fn from_index(index: usize) -> Self {
        match index {
            0 => Tab::Dashboard,
            1 => Tab::Neo,
            2 => Tab::Stacks,
            3 => Tab::Esc,
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
        &[
            PlatformView::Services,
            PlatformView::Components,
            PlatformView::Templates,
        ]
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

/// Application state - holds all data fetched from APIs
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

    // Neo conversation
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
