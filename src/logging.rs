//! Logging module using tui-logger
//!
//! Provides a TUI-integrated logging system that can be displayed in a popup widget.

use color_eyre::Result;
use log::LevelFilter;

/// Initialize the tui-logger system
pub fn init_logging() -> Result<()> {
    // Initialize tui-logger with max level Trace
    tui_logger::init_logger(LevelFilter::Trace)
        .map_err(|e| color_eyre::eyre::eyre!("Failed to init logger: {}", e))?;

    // Set default level for unknown targets to Info
    tui_logger::set_default_level(LevelFilter::Info);

    // Optionally read from RUST_LOG environment variable
    if std::env::var("RUST_LOG").is_ok() {
        tui_logger::set_env_filter_from_env(Some("RUST_LOG"));
    }

    log::info!("Lazy Pulumi started - tui-logger initialized");

    Ok(())
}
