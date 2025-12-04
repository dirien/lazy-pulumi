//! Lazy Pulumi - A stylish TUI for Pulumi Cloud, ESC, and Neo
//!
//! This application provides a terminal-based interface for managing
//! Pulumi stacks, ESC environments, and interacting with Pulumi Neo.

mod api;
mod app;
mod commands;
mod components;
mod config;
mod event;
mod logging;
mod startup;
mod theme;
mod tui;
mod ui;

use app::App;
use color_eyre::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize error handling
    color_eyre::install()?;

    // Initialize tui-logger for in-app log viewing
    logging::init_logging()?;

    // Create and run the application
    let mut app = App::new().await?;
    app.run().await?;

    Ok(())
}
