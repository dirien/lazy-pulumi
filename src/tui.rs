//! Terminal UI setup and management
//!
//! Handles terminal initialization, cleanup, and panic handling.

use color_eyre::Result;
use crossterm::{
    cursor,
    event::{DisableMouseCapture, EnableMouseCapture},
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use std::io::{self, stdout, Stdout};
use std::panic;

/// A type alias for the terminal backend
pub type Tui = Terminal<CrosstermBackend<Stdout>>;

/// Initialize the terminal
pub fn init() -> Result<Tui> {
    // Set up panic handler
    let original_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        // Restore terminal on panic
        let _ = restore();
        original_hook(panic_info);
    }));

    // Enable raw mode
    terminal::enable_raw_mode()?;

    // Enter alternate screen and enable mouse capture
    crossterm::execute!(
        stdout(),
        EnterAlternateScreen,
        EnableMouseCapture,
        cursor::Hide
    )?;

    // Create terminal
    let backend = CrosstermBackend::new(stdout());
    let terminal = Terminal::new(backend)?;

    Ok(terminal)
}

/// Restore the terminal to its original state
pub fn restore() -> Result<()> {
    terminal::disable_raw_mode()?;
    crossterm::execute!(
        io::stdout(),
        LeaveAlternateScreen,
        DisableMouseCapture,
        cursor::Show
    )?;
    Ok(())
}

/// Clear the terminal screen
#[allow(dead_code)]
pub fn clear(terminal: &mut Tui) -> Result<()> {
    terminal.clear()?;
    Ok(())
}
