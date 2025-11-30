//! Lazy Pulumi - A stylish TUI for Pulumi Cloud, ESC, and NEO
//!
//! This application provides a terminal-based interface for managing
//! Pulumi stacks, ESC environments, and interacting with Pulumi NEO.

mod api;
mod app;
mod components;
mod event;
mod theme;
mod tui;
mod ui;

use app::App;
use color_eyre::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize error handling
    color_eyre::install()?;

    // Initialize tracing for debugging (optional)
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .with_target(false)
        .init();

    // Create and run the application
    let mut app = App::new().await?;
    app.run().await?;

    Ok(())
}
