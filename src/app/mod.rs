//! Application state and main event loop
//!
//! This module contains the core application logic, state management,
//! and the main run loop. It follows the Elm Architecture (TEA) pattern:
//! - Model: AppState and App struct fields
//! - Update: handlers.rs
//! - View: render() method

mod data;
mod handlers;
mod neo;
mod types;

pub use types::{
    AppState, DataLoadResult, EscPane, FocusMode, NeoAsyncResult, PlatformView,
    SlashCommandsDialogView, Tab,
};

use color_eyre::Result;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tui_scrollview::ScrollViewState;

use crate::api::{
    EscEnvironmentSummary, NeoTask, PulumiClient, RegistryPackage, RegistryTemplate, Service, Stack,
};
use crate::commands::{
    commands_by_category, CommandCategory, CommandExecution, CommandResult, PulumiCommand,
};
use crate::components::{Spinner, StatefulList, TextEditor, TextInput};
use crate::config::Config;
use crate::event::{Event, EventHandler};
use crate::startup::{check_pulumi_cli, check_pulumi_token, StartupChecks};
use crate::theme::Theme;
use crate::tui::{self, Tui};
use crate::ui;
use tui_logger::TuiWidgetState;

/// Main application
pub struct App {
    /// Terminal instance
    terminal: Tui,

    /// Event handler
    events: EventHandler,

    /// API client
    pub(super) client: Option<PulumiClient>,

    /// Theme
    theme: Theme,

    /// Current tab
    pub(super) tab: Tab,

    /// Focus mode
    pub(super) focus: FocusMode,

    /// Show splash screen on startup
    pub(super) show_splash: bool,

    /// Whether the "don't show again" checkbox is selected
    pub(super) splash_dont_show_again: bool,

    /// Startup checks state
    pub(super) startup_checks: StartupChecks,

    /// Whether startup checks have been initiated
    pub(super) startup_checks_started: bool,

    /// User configuration
    pub(super) config: Config,

    /// Show help popup
    pub(super) show_help: bool,

    /// Show organization selector popup
    pub(super) show_org_selector: bool,

    /// Show logs popup
    pub(super) show_logs: bool,

    /// tui-logger widget state
    pub(super) logger_state: TuiWidgetState,

    /// Organization list for selector
    pub(super) org_list: StatefulList<String>,

    /// Loading state
    pub(super) is_loading: bool,

    /// Loading spinner
    pub(super) spinner: Spinner,

    /// Error message
    pub(super) error: Option<String>,

    /// Should quit
    pub(super) should_quit: bool,

    /// Application state
    pub state: AppState,

    // UI state
    pub(super) stacks_list: StatefulList<Stack>,
    pub(super) esc_list: StatefulList<EscEnvironmentSummary>,
    pub(super) neo_tasks_list: StatefulList<NeoTask>,
    pub(super) neo_input: TextInput,

    // ESC detail pane state
    /// Which pane is currently focused (Definition or Resolved Values)
    pub(super) esc_pane: types::EscPane,
    /// Scroll state for Definition pane
    pub(super) esc_definition_scroll: ScrollViewState,
    /// Scroll state for Resolved Values pane
    pub(super) esc_values_scroll: ScrollViewState,
    /// Show YAML editor dialog
    pub(super) show_esc_editor: bool,
    /// YAML editor component
    pub(super) esc_editor: TextEditor,
    /// Environment being edited (org, project, name)
    pub(super) esc_editing_env: Option<(String, String, String)>,

    // Platform UI state
    pub(super) platform_view: PlatformView,
    pub(super) services_list: StatefulList<Service>,
    pub(super) packages_list: StatefulList<RegistryPackage>,
    pub(super) templates_list: StatefulList<RegistryTemplate>,
    /// Scroll state for Component/Template description panel
    pub(super) platform_desc_scroll_state: ScrollViewState,

    /// Neo polling state - tracks if we're waiting for agent response
    pub(super) neo_polling: bool,
    /// Counter for polling interval (poll every N ticks)
    pub(super) neo_poll_counter: u8,
    /// Counter for stable polls (no new messages for N consecutive polls)
    pub(super) neo_stable_polls: u8,
    /// Previous message count (to detect changes)
    pub(super) neo_prev_message_count: usize,
    /// Max polling attempts before giving up
    pub(super) neo_max_polls: u8,
    /// Current poll count
    pub(super) neo_current_poll: u8,
    /// Background poll counter for when Neo tab is active
    pub(super) neo_bg_poll_counter: u8,
    /// Neo chat scroll view state
    pub(super) neo_scroll_state: ScrollViewState,
    /// Auto-scroll to bottom when new messages arrive
    pub(super) neo_auto_scroll: Arc<AtomicBool>,
    /// Hide task list when a task is selected (full-width chat)
    pub(super) neo_hide_task_list: bool,
    /// Show Neo task details dialog
    pub(super) show_neo_details: bool,
    /// Current task is running (from task status API)
    /// Used to keep thinking indicator visible until confirmed not running
    pub(super) neo_task_is_running: bool,
    /// Show slash command picker popup
    pub(super) neo_show_command_picker: bool,
    /// Filtered slash commands (based on input)
    pub(super) neo_filtered_commands: Vec<crate::api::NeoSlashCommand>,
    /// Selected command index in picker
    pub(super) neo_command_picker_index: usize,
    /// Pending slash commands that have been inserted but not yet sent
    pub(super) neo_pending_commands: Vec<crate::api::NeoSlashCommand>,

    // Slash commands management dialog state
    /// Show slash commands management dialog
    pub(super) show_slash_commands_dialog: bool,
    /// Current view state of the slash commands dialog
    pub(super) slash_commands_dialog_view: SlashCommandsDialogView,
    /// List of slash commands for the dialog
    pub(super) slash_commands_list: StatefulList<crate::api::NeoSlashCommand>,
    /// Selected command detail (for viewing)
    pub(super) slash_command_detail: Option<crate::api::NeoSlashCommand>,
    /// Create command form - name input
    pub(super) slash_cmd_create_name: TextInput,
    /// Create command form - description input
    pub(super) slash_cmd_create_description: TextInput,
    /// Create command form - prompt editor
    pub(super) slash_cmd_create_prompt: TextEditor,
    /// Create command form - which field is focused (0=name, 1=description, 2=prompt)
    pub(super) slash_cmd_create_focus: usize,
    /// Scroll state for command detail/prompt view
    pub(super) slash_cmd_detail_scroll: ScrollViewState,
    /// Edit command form - description input
    pub(super) slash_cmd_edit_description: TextInput,
    /// Edit command form - prompt editor
    pub(super) slash_cmd_edit_prompt: TextEditor,
    /// Edit command form - which field is focused (0=description, 1=prompt)
    pub(super) slash_cmd_edit_focus: usize,

    /// Channel for receiving async Neo results
    pub(super) neo_result_rx: mpsc::Receiver<NeoAsyncResult>,
    /// Channel sender for Neo async tasks (wrapped in Arc for cloning)
    pub(super) neo_result_tx: mpsc::Sender<NeoAsyncResult>,

    /// Channel for receiving async data loading results
    pub(super) data_result_rx: mpsc::Receiver<DataLoadResult>,
    /// Channel sender for async data loading
    pub(super) data_result_tx: mpsc::Sender<DataLoadResult>,
    /// Number of pending data load operations
    pub(super) pending_data_loads: u8,

    /// Channel for receiving async startup check results
    pub(super) startup_result_rx: mpsc::Receiver<types::StartupCheckResult>,
    /// Channel sender for async startup checks
    pub(super) startup_result_tx: mpsc::Sender<types::StartupCheckResult>,

    // Commands tab state
    /// Current view state for commands tab
    pub(super) commands_view_state: ui::CommandsViewState,
    /// List of command categories
    pub(super) commands_category_list: StatefulList<CommandCategory>,
    /// List of commands for selected category
    pub(super) commands_command_list: StatefulList<&'static PulumiCommand>,
    /// Current command execution (if any)
    pub(super) current_command_execution: Option<CommandExecution>,
    /// Parameter input fields for current command
    pub(super) commands_param_inputs: Vec<TextInput>,
    /// Currently focused parameter index
    pub(super) commands_param_focus_index: usize,
    /// Output scroll state
    pub(super) commands_output_scroll: ScrollViewState,
    /// Filter input for searching commands
    pub(super) commands_filter_input: TextInput,
    /// Whether the filter input is focused
    pub(super) commands_is_filtering: bool,
    /// Channel for receiving command execution results
    pub(super) command_result_rx: mpsc::Receiver<CommandResult>,
    /// Channel sender for command execution results
    pub(super) command_result_tx: mpsc::Sender<CommandResult>,
}

impl App {
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
                log::warn!("Failed to create API client: {}", e);
                None
            }
        };

        // Create channel for async Neo results
        let (neo_result_tx, neo_result_rx) = mpsc::channel::<NeoAsyncResult>(32);

        // Create channel for async data loading results
        let (data_result_tx, data_result_rx) = mpsc::channel::<DataLoadResult>(32);

        // Create channel for async startup check results
        let (startup_result_tx, startup_result_rx) = mpsc::channel::<types::StartupCheckResult>(4);

        // Create channel for command execution results
        let (command_result_tx, command_result_rx) = mpsc::channel::<CommandResult>(64);

        // Initialize commands category list
        let mut commands_category_list = StatefulList::new();
        commands_category_list.set_items(CommandCategory::all().to_vec());
        commands_category_list.select(Some(0));

        // Initialize commands list with first category
        let mut commands_command_list: StatefulList<&'static PulumiCommand> = StatefulList::new();
        let initial_commands = commands_by_category(CommandCategory::StackOperations);
        let has_commands = !initial_commands.is_empty();
        commands_command_list.set_items(initial_commands);
        if has_commands {
            commands_command_list.select(Some(0));
        }

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
            splash_dont_show_again: false,
            startup_checks: StartupChecks::default(),
            startup_checks_started: false,
            config,
            show_help: false,
            show_org_selector: false,
            show_logs: false,
            logger_state: TuiWidgetState::new(),
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
            esc_pane: types::EscPane::default(),
            esc_definition_scroll: ScrollViewState::default(),
            esc_values_scroll: ScrollViewState::default(),
            show_esc_editor: false,
            esc_editor: TextEditor::new(),
            esc_editing_env: None,
            platform_view: PlatformView::Services,
            services_list: StatefulList::new(),
            packages_list: StatefulList::new(),
            templates_list: StatefulList::new(),
            platform_desc_scroll_state: ScrollViewState::default(),
            neo_polling: false,
            neo_poll_counter: 0,
            neo_stable_polls: 0,
            neo_prev_message_count: 0,
            neo_max_polls: 60, // Max 60 polls (~60 seconds at 1 poll/second)
            neo_current_poll: 0,
            neo_bg_poll_counter: 0,
            neo_scroll_state: ScrollViewState::default(),
            neo_auto_scroll: Arc::new(AtomicBool::new(true)),
            neo_hide_task_list: false,
            show_neo_details: false,
            neo_task_is_running: false,
            neo_show_command_picker: false,
            neo_filtered_commands: Vec::new(),
            neo_command_picker_index: 0,
            neo_pending_commands: Vec::new(),
            // Slash commands management dialog
            show_slash_commands_dialog: false,
            slash_commands_dialog_view: SlashCommandsDialogView::default(),
            slash_commands_list: StatefulList::new(),
            slash_command_detail: None,
            slash_cmd_create_name: TextInput::new(),
            slash_cmd_create_description: TextInput::new(),
            slash_cmd_create_prompt: TextEditor::new(),
            slash_cmd_create_focus: 0,
            slash_cmd_detail_scroll: ScrollViewState::default(),
            slash_cmd_edit_description: TextInput::new(),
            slash_cmd_edit_prompt: TextEditor::new(),
            slash_cmd_edit_focus: 0,
            neo_result_rx,
            neo_result_tx,
            data_result_rx,
            data_result_tx,
            pending_data_loads: 0,
            startup_result_rx,
            startup_result_tx,
            // Commands tab state
            commands_view_state: ui::CommandsViewState::default(),
            commands_category_list,
            commands_command_list,
            current_command_execution: None,
            commands_param_inputs: Vec::new(),
            commands_param_focus_index: 0,
            commands_output_scroll: ScrollViewState::default(),
            commands_filter_input: TextInput::new(),
            commands_is_filtering: false,
            command_result_rx,
            command_result_tx,
        };

        // If splash is not shown, run startup checks and load data immediately
        if !show_splash {
            // Run startup checks synchronously
            app.startup_checks.token_check.status = check_pulumi_token();
            app.startup_checks.cli_check.status = check_pulumi_cli().await;
            app.startup_checks_started = true;

            // Only load data if checks passed
            if app.startup_checks.all_passed() {
                app.load_initial_data().await;
            }
        }

        Ok(app)
    }

    /// Main run loop
    pub async fn run(&mut self) -> Result<()> {
        while !self.should_quit {
            // Spawn startup checks if showing splash and not started yet
            if self.show_splash && !self.startup_checks_started {
                self.spawn_startup_checks();
            }

            // Render
            self.render()?;

            // Check for async startup check results (non-blocking)
            self.process_startup_results().await;

            // Check for async data loading results (non-blocking)
            self.process_data_results();

            // Check for async Neo results (non-blocking)
            self.process_neo_results();

            // Check for async command results (non-blocking)
            self.process_command_results();

            // Handle events
            match self.events.next().await? {
                Event::Tick => {
                    self.spinner.tick();

                    // Process command results on every tick for responsive streaming output
                    self.process_command_results();

                    // Poll for Neo updates if we're waiting for a response (fast polling)
                    if self.neo_polling {
                        self.neo_poll_counter += 1;
                        // Poll every 5 ticks (~500ms at 100ms tick rate)
                        if self.neo_poll_counter >= 5 {
                            self.neo_poll_counter = 0;
                            self.spawn_neo_poll();
                        }
                    }
                    // Background polling when Neo tab is active with a task selected
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

    /// Render the UI
    pub(super) fn render(&mut self) -> Result<()> {
        // Extract values before the closure to avoid borrow issues
        let theme = &self.theme;
        let tab = self.tab;
        let org = self.state.organization.as_deref();
        let show_splash = self.show_splash;
        let splash_dont_show_again = self.splash_dont_show_again;
        let startup_checks = self.startup_checks.clone();
        let show_help = self.show_help;
        let show_org_selector = self.show_org_selector;
        let show_logs = self.show_logs;
        let show_neo_details = self.show_neo_details;
        let show_esc_editor = self.show_esc_editor;
        let show_slash_commands_dialog = self.show_slash_commands_dialog;
        let slash_commands_dialog_view = self.slash_commands_dialog_view;
        let esc_editor = &self.esc_editor;
        let esc_editing_env = self.esc_editing_env.clone();
        let logger_state = &self.logger_state;
        let is_loading = self.is_loading;
        // For Neo tab, show spinner when polling (waiting for response)
        // Also show if task status indicates it's still running (even if polling stopped)
        let neo_is_thinking = self.neo_polling || self.is_loading || self.neo_task_is_running;
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
        let neo_hide_task_list = self.neo_hide_task_list;
        let neo_show_command_picker = self.neo_show_command_picker;
        let neo_filtered_commands = &self.neo_filtered_commands;
        let neo_command_picker_index = self.neo_command_picker_index;
        let neo_pending_commands = &self.neo_pending_commands;

        // Slash commands dialog state
        let slash_commands_list = &mut self.slash_commands_list;
        let slash_command_detail = self.slash_command_detail.as_ref();
        let slash_cmd_create_name = &self.slash_cmd_create_name;
        let slash_cmd_create_description = &self.slash_cmd_create_description;
        let slash_cmd_create_prompt = &self.slash_cmd_create_prompt;
        let slash_cmd_create_focus = self.slash_cmd_create_focus;
        let slash_cmd_detail_scroll = &mut self.slash_cmd_detail_scroll;
        let slash_cmd_edit_description = &self.slash_cmd_edit_description;
        let slash_cmd_edit_prompt = &self.slash_cmd_edit_prompt;
        let slash_cmd_edit_focus = self.slash_cmd_edit_focus;

        // ESC detail pane state
        let esc_pane = self.esc_pane;
        let esc_definition_scroll = &mut self.esc_definition_scroll;
        let esc_values_scroll = &mut self.esc_values_scroll;

        // Platform state
        let platform_view = self.platform_view;
        let services_list = &mut self.services_list;
        let packages_list = &mut self.packages_list;
        let templates_list = &mut self.templates_list;
        let platform_desc_scroll_state = &mut self.platform_desc_scroll_state;

        // Commands state
        let commands_view_state = self.commands_view_state;
        let commands_category_list = &mut self.commands_category_list;
        let commands_command_list = &mut self.commands_command_list;
        let current_command_execution = self.current_command_execution.as_ref();
        let commands_param_inputs = &self.commands_param_inputs;
        let commands_param_focus_index = self.commands_param_focus_index;
        let commands_output_scroll = &mut self.commands_output_scroll;
        let commands_filter_input = &self.commands_filter_input;
        let commands_is_filtering = self.commands_is_filtering;

        self.terminal.draw(|frame| {
            // Get selected task for details dialog (cloned inside closure)
            let selected_task_for_details: Option<NeoTask> = if show_neo_details {
                // First try to use the current task if one is loaded
                if let Some(ref task_id) = state.current_task_id {
                    state.neo_tasks.iter().find(|t| &t.id == task_id).cloned()
                } else {
                    // Fall back to selected task in list
                    neo_tasks_list.selected().cloned()
                }
            } else {
                None
            };

            // Show splash screen with startup checklist
            if show_splash {
                ui::render_splash(
                    frame,
                    theme,
                    spinner_char,
                    splash_dont_show_again,
                    &startup_checks,
                );
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
                        ui::EscViewProps {
                            environments: esc_list,
                            selected_env_yaml: state.selected_env_yaml.as_deref(),
                            selected_env_yaml_highlighted: state
                                .selected_env_yaml_highlighted
                                .as_ref(),
                            selected_env_values: state.selected_env_values.as_ref(),
                            selected_env_values_highlighted: state
                                .selected_env_values_highlighted
                                .as_ref(),
                            focused_pane: esc_pane,
                            definition_scroll: esc_definition_scroll,
                            values_scroll: esc_values_scroll,
                        },
                    );
                }
                Tab::Neo => {
                    ui::render_neo_view(
                        frame,
                        theme,
                        content_area,
                        ui::NeoViewProps {
                            tasks: neo_tasks_list,
                            messages: &state.neo_messages,
                            input: neo_input,
                            scroll_state: neo_scroll_state,
                            auto_scroll: &neo_auto_scroll,
                            is_loading: neo_is_thinking,
                            spinner_char,
                            hide_task_list: neo_hide_task_list,
                            command_picker: ui::CommandPickerProps {
                                show: neo_show_command_picker,
                                filtered_commands: neo_filtered_commands,
                                index: neo_command_picker_index,
                                all_commands: &state.neo_slash_commands,
                                pending_commands: neo_pending_commands,
                            },
                        },
                    );
                }
                Tab::Platform => {
                    ui::render_platform_view(
                        frame,
                        theme,
                        content_area,
                        ui::PlatformViewProps {
                            current_view: platform_view,
                            services: services_list,
                            packages: packages_list,
                            templates: templates_list,
                            description_scroll_state: platform_desc_scroll_state,
                        },
                    );
                }
                Tab::Commands => {
                    ui::render_commands_view(
                        frame,
                        theme,
                        content_area,
                        ui::CommandsViewProps {
                            view_state: commands_view_state,
                            category_list: commands_category_list,
                            command_list: commands_command_list,
                            current_execution: current_command_execution,
                            param_inputs: commands_param_inputs,
                            param_focus_index: commands_param_focus_index,
                            output_scroll: commands_output_scroll,
                            filter_input: commands_filter_input,
                            is_filtering: commands_is_filtering,
                        },
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
                ui::render_logs(frame, theme, logger_state);
            }

            // Neo task details popup
            if show_neo_details {
                if let Some(ref task) = selected_task_for_details {
                    ui::render_neo_details_dialog(frame, theme, task);
                }
            }

            // ESC YAML editor popup
            if show_esc_editor {
                let env_name = esc_editing_env
                    .as_ref()
                    .map(|(_, p, n)| format!("{}/{}", p, n))
                    .unwrap_or_else(|| "Unknown".to_string());
                ui::render_esc_editor(frame, theme, esc_editor, &env_name);
            }

            // Slash commands management dialog
            if show_slash_commands_dialog {
                ui::render_slash_commands_dialog(
                    frame,
                    theme,
                    ui::SlashCommandsDialogProps {
                        view: slash_commands_dialog_view,
                        commands: slash_commands_list,
                        selected_detail: slash_command_detail,
                        create_name: slash_cmd_create_name,
                        create_description: slash_cmd_create_description,
                        create_prompt: slash_cmd_create_prompt,
                        create_focus: slash_cmd_create_focus,
                        detail_scroll: slash_cmd_detail_scroll,
                        edit_description: slash_cmd_edit_description,
                        edit_prompt: slash_cmd_edit_prompt,
                        edit_focus: slash_cmd_edit_focus,
                    },
                );
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

        if self.show_neo_details {
            return "Press d or Esc to close details".to_string();
        }

        if self.show_slash_commands_dialog {
            return match self.slash_commands_dialog_view {
                SlashCommandsDialogView::List => {
                    "↑↓: navigate | Enter: view | n: new | e: edit | d: delete (custom) | Esc: close"
                        .to_string()
                }
                SlashCommandsDialogView::Detail => {
                    "j/k: scroll | e: edit | Esc: back to list".to_string()
                }
                SlashCommandsDialogView::Create => {
                    "Tab: next field | Shift+Tab: prev | Ctrl+S: create | Esc: cancel".to_string()
                }
                SlashCommandsDialogView::Edit => {
                    "Tab: next field | Shift+Tab: prev | Ctrl+S: save | Esc: cancel".to_string()
                }
                SlashCommandsDialogView::ConfirmDelete => {
                    "y: confirm delete | n/Esc: cancel".to_string()
                }
            };
        }

        if self.show_esc_editor {
            return "Esc: Save & Close | Ctrl+C: Cancel | Tab: Indent | Ctrl+D: Delete line"
                .to_string();
        }

        if self.show_logs {
            return "h:targets | f:focus | +/-:capture | </>:show | PgUp/Dn:scroll | Space:hide off | l/Esc:close"
                .to_string();
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
                Tab::Dashboard => {
                    "Tab: switch | o: org | l: logs | ?: help | r: refresh | q: quit".to_string()
                }
                Tab::Stacks => {
                    "↑↓: navigate | o: org | l: logs | Enter: details | r: refresh | q: quit"
                        .to_string()
                }
                Tab::Esc => {
                    "↑↓: envs | ←→: panes | j/k: scroll | Enter: load | o: resolve | e: edit | q: quit"
                        .to_string()
                }
                Tab::Neo => {
                    if self.neo_hide_task_list {
                        "j/k: scroll | /: commands | c: cmds | d: details | n: new | i: type | Esc: tasks | q: quit"
                            .to_string()
                    } else {
                        "↑↓: tasks | Enter: select | /: commands | c: cmds | n: new | i: type | q: quit".to_string()
                    }
                }
                Tab::Platform => {
                    "↑↓: navigate | ←→: switch view | o: org | l: logs | r: refresh | q: quit"
                        .to_string()
                }
                Tab::Commands => {
                    match self.commands_view_state {
                        ui::CommandsViewState::BrowsingCategories => {
                            "↑↓: categories | →/Enter: commands | /: filter | q: quit".to_string()
                        }
                        ui::CommandsViewState::BrowsingCommands => {
                            "↑↓: commands | ←: categories | Enter: run | /: filter | q: quit".to_string()
                        }
                        ui::CommandsViewState::InputDialog => {
                            "Tab: next field | Enter: run | Esc: cancel".to_string()
                        }
                        ui::CommandsViewState::ConfirmDialog => {
                            "y: confirm | n/Esc: cancel".to_string()
                        }
                        ui::CommandsViewState::OutputView => {
                            "j/k: scroll | Esc: close | q: quit".to_string()
                        }
                    }
                }
            },
        }
    }

    /// Process command execution results (non-blocking)
    fn process_command_results(&mut self) {
        use crate::commands::update_execution_state;

        while let Ok(result) = self.command_result_rx.try_recv() {
            if let Some(ref mut execution) = self.current_command_execution {
                update_execution_state(execution, result);
            }
        }
    }
}
