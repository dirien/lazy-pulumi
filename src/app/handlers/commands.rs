//! Commands tab handler

use crossterm::event::KeyEvent;
use tui_scrollview::ScrollViewState;

use crate::app::App;
use crate::commands::{
    can_run_command, commands_by_category, spawn_command, CommandExecution, CommandExecutionState,
    ExecutionMode,
};
use crate::event::keys;
use crate::ui::CommandsViewState;

impl App {
    /// Handle Commands view keys
    pub(crate) async fn handle_commands_key(&mut self, key: KeyEvent) {
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
