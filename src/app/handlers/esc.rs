//! ESC (Environments, Secrets, Configuration) tab handler

use crossterm::event::KeyEvent;
use tui_scrollview::ScrollViewState;

use crate::app::types::EscPane;
use crate::app::App;
use crate::event::keys;
use crate::ui::syntax::highlight_yaml;
use crate::ui::{extract_values, json_to_yaml};

impl App {
    /// Handle ESC view keys
    pub(crate) async fn handle_esc_key(&mut self, key: KeyEvent) {
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
    pub(crate) async fn handle_esc_editor_key(&mut self, key: KeyEvent) {
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
}
