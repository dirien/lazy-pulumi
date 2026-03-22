//! Startup checks and splash screen handler

use crossterm::event::{KeyCode, KeyEvent};

use crate::app::App;
use crate::startup::{check_pulumi_cli, check_pulumi_token, CheckStatus};

impl App {
    /// Spawn startup checks as background tasks (non-blocking)
    /// This allows the event loop to continue and the spinner to animate
    pub(crate) fn spawn_startup_checks(&mut self) {
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
                .send(crate::app::types::StartupCheckResult::TokenCheck(status))
                .await;
        });

        // Spawn CLI check (async)
        let tx = self.startup_result_tx.clone();
        tokio::spawn(async move {
            let status = check_pulumi_cli().await;
            let _ = tx
                .send(crate::app::types::StartupCheckResult::CliCheck(status))
                .await;
        });
    }

    /// Process startup check results (non-blocking)
    pub(crate) async fn process_startup_results(&mut self) {
        // Try to receive all pending results without blocking
        while let Ok(result) = self.startup_result_rx.try_recv() {
            match result {
                crate::app::types::StartupCheckResult::TokenCheck(status) => {
                    self.startup_checks.token_check.status = status;
                }
                crate::app::types::StartupCheckResult::CliCheck(status) => {
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
    pub(crate) fn handle_splash_key(&mut self, key: KeyEvent) {
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
    pub(crate) fn dismiss_splash(&mut self) {
        self.show_splash = false;

        // Save preference if "don't show again" is checked
        if self.splash_dont_show_again {
            self.config.show_splash = false;
            self.config.save();
        }
    }
}
