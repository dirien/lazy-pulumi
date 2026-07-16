//! Application types and state definitions
//!
//! This module contains the core type definitions used throughout the application,
//! including enums for tabs, focus modes, and the main application state struct.

use crate::api::{
    EscEnvironmentSummary, NeoMessage, NeoSlashCommand, NeoTask, OrgStackUpdate, RegistryPackage,
    RegistryTemplate, ResourceSummaryPoint, Service, Stack,
};

/// Async data loading result
#[derive(Debug)]
pub enum DataLoadResult {
    Stacks(Vec<Stack>),
    EscEnvironments(Vec<EscEnvironmentSummary>),
    NeoTasks(Vec<NeoTask>),
    NeoSlashCommands(Vec<NeoSlashCommand>),
    /// Total resource count for the dashboard tile
    ResourceCount(i64),
    Services(Vec<Service>),
    PrivatePackages(Vec<RegistryPackage>),
    RegistryPackages(Vec<RegistryPackage>),
    RegistryTemplates(Vec<RegistryTemplate>),
    /// Recent stack updates across the organization
    RecentUpdates(Vec<OrgStackUpdate>),
    /// Resource count over time (for dashboard chart)
    ResourceSummary(Vec<ResourceSummaryPoint>),
    /// README content loaded for a package (key, content)
    ReadmeContent {
        package_key: String,
        content: String,
    },
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
    Commands,
}

impl Tab {
    pub fn all() -> &'static [Tab] {
        // Dashboard, Commands, Neo, then the rest
        &[
            Tab::Dashboard,
            Tab::Commands,
            Tab::Neo,
            Tab::Stacks,
            Tab::Esc,
            Tab::Platform,
        ]
    }

    pub fn title(&self) -> &'static str {
        match self {
            Tab::Dashboard => " Dashboard ",
            Tab::Stacks => " Stacks ",
            Tab::Esc => " Environment ",
            Tab::Neo => " Neo ",
            Tab::Platform => " Platform ",
            Tab::Commands => " Commands ",
        }
    }

    pub fn index(&self) -> usize {
        match self {
            Tab::Dashboard => 0,
            Tab::Commands => 1,
            Tab::Neo => 2,
            Tab::Stacks => 3,
            Tab::Esc => 4,
            Tab::Platform => 5,
        }
    }

    pub fn from_index(index: usize) -> Self {
        match index {
            0 => Tab::Dashboard,
            1 => Tab::Commands,
            2 => Tab::Neo,
            3 => Tab::Stacks,
            4 => Tab::Esc,
            5 => Tab::Platform,
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

/// ESC detail pane selection (which content pane is focused)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EscPane {
    #[default]
    Definition,
    ResolvedValues,
}

impl EscPane {
    #[allow(dead_code)]
    pub fn toggle(&self) -> Self {
        match self {
            EscPane::Definition => EscPane::ResolvedValues,
            EscPane::ResolvedValues => EscPane::Definition,
        }
    }
}

/// Platform sub-view selection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlatformView {
    Services,
    PrivateComponents,
    Registry,
    Templates,
}

/// Slash commands management dialog view state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SlashCommandsDialogView {
    /// Browsing list of commands
    #[default]
    List,
    /// Viewing command details
    Detail,
    /// Creating a new command
    Create,
    /// Editing an existing command
    Edit,
    /// Confirming deletion
    ConfirmDelete,
}

impl PlatformView {
    pub fn all() -> &'static [PlatformView] {
        &[
            PlatformView::Services,
            PlatformView::PrivateComponents,
            PlatformView::Registry,
            PlatformView::Templates,
        ]
    }

    pub fn title(&self) -> &'static str {
        match self {
            PlatformView::Services => "Services",
            PlatformView::PrivateComponents => "Private Components",
            PlatformView::Registry => "Registry",
            PlatformView::Templates => "Templates",
        }
    }

    pub fn index(&self) -> usize {
        match self {
            PlatformView::Services => 0,
            PlatformView::PrivateComponents => 1,
            PlatformView::Registry => 2,
            PlatformView::Templates => 3,
        }
    }

    pub fn from_index(index: usize) -> Self {
        match index {
            0 => PlatformView::Services,
            1 => PlatformView::PrivateComponents,
            2 => PlatformView::Registry,
            3 => PlatformView::Templates,
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

/// Neo task approval mode for new task creation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NeoApprovalMode {
    /// Use org default (Manual) — don't send the field
    #[default]
    Default,
    Manual,
    Auto,
    Balanced,
}

impl NeoApprovalMode {
    pub fn cycle(&self) -> Self {
        match self {
            Self::Default => Self::Manual,
            Self::Manual => Self::Auto,
            Self::Auto => Self::Balanced,
            Self::Balanced => Self::Default,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Default => "Manual (org default)",
            Self::Manual => "Manual",
            Self::Auto => "Auto",
            Self::Balanced => "Balanced",
        }
    }

    /// Returns the API value to send, or None if using org default
    pub fn api_value(&self) -> Option<&'static str> {
        match self {
            Self::Default => None,
            Self::Manual => Some("manual"),
            Self::Auto => Some("auto"),
            Self::Balanced => Some("balanced"),
        }
    }
}

/// Neo task permission mode for new task creation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NeoPermissionMode {
    /// Use org default (Full) — don't send the field
    #[default]
    Default,
    Full,
    ReadOnly,
}

impl NeoPermissionMode {
    pub fn cycle(&self) -> Self {
        match self {
            Self::Default => Self::Full,
            Self::Full => Self::ReadOnly,
            Self::ReadOnly => Self::Default,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Default => "Full (org default)",
            Self::Full => "Full",
            Self::ReadOnly => "Read-only",
        }
    }

    /// Returns the API value to send, or None if using org default
    pub fn api_value(&self) -> Option<&'static str> {
        match self {
            Self::Default => None,
            Self::Full => Some("default"),
            Self::ReadOnly => Some("read-only"),
        }
    }
}

/// Neo task plan mode for new task creation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NeoPlanMode {
    /// Use org default (Off) — don't send the field
    #[default]
    Default,
    On,
    Off,
}

impl NeoPlanMode {
    pub fn cycle(&self) -> Self {
        match self {
            Self::Default => Self::On,
            Self::On => Self::Off,
            Self::Off => Self::Default,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Default => "Off (org default)",
            Self::On => "On",
            Self::Off => "Off",
        }
    }

    /// Returns the API value to send, or None if using org default
    pub fn api_value(&self) -> Option<bool> {
        match self {
            Self::Default => None,
            Self::On => Some(true),
            Self::Off => Some(false),
        }
    }
}

/// Settings for creating a new Neo task
#[derive(Debug, Clone, Copy, Default)]
pub struct NeoTaskSettings {
    pub approval_mode: NeoApprovalMode,
    pub permission_mode: NeoPermissionMode,
    pub plan_mode: NeoPlanMode,
}

impl NeoTaskSettings {
    /// Returns true if any setting differs from org default
    #[allow(dead_code)]
    pub fn has_overrides(&self) -> bool {
        self.approval_mode != NeoApprovalMode::Default
            || self.permission_mode != NeoPermissionMode::Default
            || self.plan_mode != NeoPlanMode::Default
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── NeoApprovalMode ──

    #[test]
    fn approval_mode_cycle_returns_to_default() {
        let m = NeoApprovalMode::Default;
        let m = m.cycle(); // Manual
        let m = m.cycle(); // Auto
        let m = m.cycle(); // Balanced
        let m = m.cycle(); // Default
        assert_eq!(m, NeoApprovalMode::Default);
    }

    #[test]
    fn approval_mode_default_label_shows_value() {
        assert_eq!(NeoApprovalMode::Default.label(), "Manual (org default)");
    }

    #[test]
    fn approval_mode_default_api_value_is_none() {
        assert_eq!(NeoApprovalMode::Default.api_value(), None);
    }

    #[test]
    fn approval_mode_explicit_api_values() {
        assert_eq!(NeoApprovalMode::Manual.api_value(), Some("manual"));
        assert_eq!(NeoApprovalMode::Auto.api_value(), Some("auto"));
        assert_eq!(NeoApprovalMode::Balanced.api_value(), Some("balanced"));
    }

    // ── NeoPermissionMode ──

    #[test]
    fn permission_mode_cycle_returns_to_default() {
        let m = NeoPermissionMode::Default;
        let m = m.cycle(); // Full
        let m = m.cycle(); // ReadOnly
        let m = m.cycle(); // Default
        assert_eq!(m, NeoPermissionMode::Default);
    }

    #[test]
    fn permission_mode_default_label_shows_value() {
        assert_eq!(NeoPermissionMode::Default.label(), "Full (org default)");
    }

    #[test]
    fn permission_mode_default_api_value_is_none() {
        assert_eq!(NeoPermissionMode::Default.api_value(), None);
    }

    #[test]
    fn permission_mode_explicit_api_values() {
        assert_eq!(NeoPermissionMode::Full.api_value(), Some("default"));
        assert_eq!(NeoPermissionMode::ReadOnly.api_value(), Some("read-only"));
    }

    // ── NeoPlanMode ──

    #[test]
    fn plan_mode_cycle_returns_to_default() {
        let m = NeoPlanMode::Default;
        let m = m.cycle(); // On
        let m = m.cycle(); // Off
        let m = m.cycle(); // Default
        assert_eq!(m, NeoPlanMode::Default);
    }

    #[test]
    fn plan_mode_default_label_shows_value() {
        assert_eq!(NeoPlanMode::Default.label(), "Off (org default)");
    }

    #[test]
    fn plan_mode_default_api_value_is_none() {
        assert_eq!(NeoPlanMode::Default.api_value(), None);
    }

    #[test]
    fn plan_mode_explicit_api_values() {
        assert_eq!(NeoPlanMode::On.api_value(), Some(true));
        assert_eq!(NeoPlanMode::Off.api_value(), Some(false));
    }

    // ── NeoTaskSettings ──

    #[test]
    fn settings_default_has_no_overrides() {
        let s = NeoTaskSettings::default();
        assert!(!s.has_overrides());
    }

    #[test]
    fn settings_with_changed_approval_has_overrides() {
        let mut s = NeoTaskSettings::default();
        s.approval_mode = NeoApprovalMode::Auto;
        assert!(s.has_overrides());
    }

    #[test]
    fn settings_with_plan_mode_on_has_overrides() {
        let mut s = NeoTaskSettings::default();
        s.plan_mode = NeoPlanMode::On;
        assert!(s.has_overrides());
    }
}

/// Application state - holds all data fetched from APIs
#[derive(Default)]
pub struct AppState {
    // Data
    pub stacks: Vec<Stack>,
    pub esc_environments: Vec<EscEnvironmentSummary>,
    pub neo_tasks: Vec<NeoTask>,
    /// Total resource count (None while loading)
    pub resource_count: Option<i64>,
    /// Per-tile loading markers for the dashboard (set while a refresh is in flight)
    pub stacks_loading: bool,
    pub envs_loading: bool,
    pub neo_tasks_loading: bool,
    /// Recent stack updates across the organization (for dashboard)
    pub recent_updates: Vec<OrgStackUpdate>,
    /// Resource count over time (for dashboard chart)
    pub resource_summary: Vec<ResourceSummaryPoint>,

    // Selected stack details
    pub selected_stack_updates: Vec<(i32, String, String)>,

    // Selected ESC env details
    pub selected_env_yaml: Option<String>,
    /// Cached syntax-highlighted lines for YAML definition (computed once when yaml changes)
    pub selected_env_yaml_highlighted: Option<Vec<ratatui::text::Line<'static>>>,
    pub selected_env_values: Option<serde_json::Value>,
    /// Cached syntax-highlighted lines for resolved values (computed once when values change)
    pub selected_env_values_highlighted: Option<Vec<ratatui::text::Line<'static>>>,

    // Neo conversation
    pub neo_messages: Vec<NeoMessage>,
    pub current_task_id: Option<String>,
    /// Available slash commands from API
    pub neo_slash_commands: Vec<NeoSlashCommand>,

    // Platform data
    pub services: Vec<Service>,
    pub private_packages: Vec<RegistryPackage>,
    pub registry_packages: Vec<RegistryPackage>,
    pub registry_templates: Vec<RegistryTemplate>,

    // Organization
    pub organization: Option<String>,
    pub organizations: Vec<String>,
}
