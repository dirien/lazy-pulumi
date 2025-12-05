//! Data loading and refresh logic
//!
//! This module handles all async data loading operations including
//! initial data fetching, refreshing, and processing results from async tasks.

use std::process::Stdio;
use tokio::process::Command;

use super::types::DataLoadResult;
use super::App;

impl App {
    /// Get the default organization from pulumi CLI
    pub(super) async fn get_default_org() -> Option<String> {
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
    pub(super) fn spawn_set_default_org(org: String) {
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

    /// Load initial data
    pub(super) async fn load_initial_data(&mut self) {
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
    pub(super) fn refresh_data(&mut self) {
        if let Some(ref client) = self.client {
            let org = self.state.organization.clone();
            let tx = self.data_result_tx.clone();

            // Track how many loads we're starting (now 10 with slash commands)
            self.pending_data_loads = 10;
            self.is_loading = true;
            self.spinner.set_message("Loading data...");

            // Spawn all data loads in parallel
            let client1 = client.clone();
            let org1 = org.clone();
            let tx1 = tx.clone();
            tokio::spawn(async move {
                match client1.list_stacks(org1.as_deref()).await {
                    Ok(stacks) => {
                        let _ = tx1.send(DataLoadResult::Stacks(stacks)).await;
                    }
                    Err(e) => {
                        let _ = tx1
                            .send(DataLoadResult::Error(format!("Stacks: {}", e)))
                            .await;
                    }
                }
            });

            let client2 = client.clone();
            let org2 = org.clone();
            let tx2 = tx.clone();
            tokio::spawn(async move {
                match client2.list_esc_environments(org2.as_deref()).await {
                    Ok(envs) => {
                        let _ = tx2.send(DataLoadResult::EscEnvironments(envs)).await;
                    }
                    Err(e) => {
                        let _ = tx2
                            .send(DataLoadResult::Error(format!("ESC: {}", e)))
                            .await;
                    }
                }
            });

            let client3 = client.clone();
            let org3 = org.clone();
            let tx3 = tx.clone();
            tokio::spawn(async move {
                match client3.list_neo_tasks(org3.as_deref()).await {
                    Ok(tasks) => {
                        let _ = tx3.send(DataLoadResult::NeoTasks(tasks)).await;
                    }
                    Err(e) => {
                        let _ = tx3
                            .send(DataLoadResult::Error(format!("Neo: {}", e)))
                            .await;
                    }
                }
            });

            let client4 = client.clone();
            let org4 = org.clone();
            let tx4 = tx.clone();
            tokio::spawn(async move {
                match client4.search_resources(org4.as_deref(), "").await {
                    Ok(resources) => {
                        let _ = tx4.send(DataLoadResult::Resources(resources)).await;
                    }
                    Err(e) => {
                        let _ = tx4
                            .send(DataLoadResult::Error(format!("Resources: {}", e)))
                            .await;
                    }
                }
            });

            let client5 = client.clone();
            let org5 = org.clone();
            let tx5 = tx.clone();
            tokio::spawn(async move {
                match client5.list_services(org5.as_deref()).await {
                    Ok(services) => {
                        let _ = tx5.send(DataLoadResult::Services(services)).await;
                    }
                    Err(e) => {
                        let _ = tx5
                            .send(DataLoadResult::Error(format!("Services: {}", e)))
                            .await;
                    }
                }
            });

            let client6 = client.clone();
            let org6 = org.clone();
            let tx6 = tx.clone();
            tokio::spawn(async move {
                match client6.list_registry_packages(org6.as_deref()).await {
                    Ok(packages) => {
                        let _ = tx6.send(DataLoadResult::RegistryPackages(packages)).await;
                    }
                    Err(e) => {
                        let _ = tx6
                            .send(DataLoadResult::Error(format!("Packages: {}", e)))
                            .await;
                    }
                }
            });

            let client7 = client.clone();
            let org7 = org.clone();
            let tx7 = tx.clone();
            tokio::spawn(async move {
                match client7.list_registry_templates(org7.as_deref()).await {
                    Ok(templates) => {
                        let _ = tx7.send(DataLoadResult::RegistryTemplates(templates)).await;
                    }
                    Err(e) => {
                        let _ = tx7
                            .send(DataLoadResult::Error(format!("Templates: {}", e)))
                            .await;
                    }
                }
            });

            // Load recent stack updates (for dashboard)
            let client8 = client.clone();
            let org8 = org.clone();
            let tx8 = tx.clone();
            tokio::spawn(async move {
                match client8.get_org_recent_updates(org8.as_deref(), 15).await {
                    Ok(updates) => {
                        let _ = tx8.send(DataLoadResult::RecentUpdates(updates)).await;
                    }
                    Err(e) => {
                        let _ = tx8
                            .send(DataLoadResult::Error(format!("Recent updates: {}", e)))
                            .await;
                    }
                }
            });

            // Load resource summary for dashboard chart (last 30 days)
            let client9 = client.clone();
            let org9 = org.clone();
            let tx9 = tx.clone();
            tokio::spawn(async move {
                match client9.get_resource_summary(org9.as_deref(), "daily", 30).await {
                    Ok(summary) => {
                        let _ = tx9.send(DataLoadResult::ResourceSummary(summary)).await;
                    }
                    Err(e) => {
                        let _ = tx9
                            .send(DataLoadResult::Error(format!("Resource summary: {}", e)))
                            .await;
                    }
                }
            });

            // Load Neo slash commands
            let client10 = client.clone();
            let org10 = org;
            let tx10 = tx;
            tokio::spawn(async move {
                if let Some(org) = org10 {
                    match client10.get_neo_slash_commands(&org).await {
                        Ok(commands) => {
                            let _ = tx10.send(DataLoadResult::NeoSlashCommands(commands)).await;
                        }
                        Err(e) => {
                            log::debug!("Neo slash commands: {} (may not be available)", e);
                            // Send empty list on error - slash commands are optional
                            let _ = tx10.send(DataLoadResult::NeoSlashCommands(vec![])).await;
                        }
                    }
                } else {
                    let _ = tx10.send(DataLoadResult::NeoSlashCommands(vec![])).await;
                }
            });
        }
    }

    /// Process async data loading results (non-blocking)
    pub(super) fn process_data_results(&mut self) {
        while let Ok(result) = self.data_result_rx.try_recv() {
            self.pending_data_loads = self.pending_data_loads.saturating_sub(1);

            match result {
                DataLoadResult::Stacks(stacks) => {
                    self.state.stacks = stacks.clone();
                    self.stacks_list.set_items(stacks);
                }
                DataLoadResult::EscEnvironments(envs) => {
                    log::info!("Received {} ESC environments", envs.len());
                    self.state.esc_environments = envs.clone();
                    self.esc_list.set_items(envs);
                }
                DataLoadResult::NeoTasks(tasks) => {
                    self.state.neo_tasks = tasks.clone();
                    self.neo_tasks_list.set_items(tasks);
                }
                DataLoadResult::NeoSlashCommands(commands) => {
                    log::info!("Received {} Neo slash commands", commands.len());
                    self.state.neo_slash_commands = commands;
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
                DataLoadResult::RecentUpdates(updates) => {
                    self.state.recent_updates = updates;
                }
                DataLoadResult::ResourceSummary(summary) => {
                    self.state.resource_summary = summary;
                }
                DataLoadResult::ReadmeContent {
                    package_key,
                    content,
                } => {
                    // Find the package and update its readme_content
                    if let Some(pkg) = self
                        .packages_list
                        .items_mut()
                        .iter_mut()
                        .find(|p| p.key() == package_key)
                    {
                        pkg.readme_content = Some(content);
                    }
                }
                DataLoadResult::Error(e) => {
                    log::warn!("Data load error: {}", e);
                }
            }

            // Clear loading state when all loads complete
            if self.pending_data_loads == 0 {
                self.is_loading = false;
                // Note: splash screen is now dismissed via user interaction, not auto-hide
            }
        }
    }

    /// Load README for the currently selected package (if not already loaded)
    pub(super) fn spawn_readme_load_for_selected_package(&self) {
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
                            let _ = tx
                                .send(DataLoadResult::ReadmeContent {
                                    package_key,
                                    content,
                                })
                                .await;
                        }
                        Err(e) => {
                            log::debug!("Failed to load README: {}", e);
                        }
                    }
                });
            }
        }
    }
}
