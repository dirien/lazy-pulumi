//! File-based logging module
//!
//! Redirects tracing output to a file to avoid interfering with the TUI.

use color_eyre::Result;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::sync::OnceLock;
use tracing_subscriber::prelude::*;

/// Global log file path
static LOG_FILE_PATH: OnceLock<PathBuf> = OnceLock::new();

/// Get the log file path
pub fn log_file_path() -> PathBuf {
    LOG_FILE_PATH
        .get()
        .cloned()
        .unwrap_or_else(|| {
            directories::BaseDirs::new()
                .map(|dirs| dirs.cache_dir().join("lazy-pulumi").join("app.log"))
                .unwrap_or_else(|| PathBuf::from("/tmp/lazy-pulumi.log"))
        })
}

/// Initialize file-based logging
pub fn init_file_logging() -> Result<()> {
    // Determine log file path
    let log_path = directories::BaseDirs::new()
        .map(|dirs| {
            let cache_dir = dirs.cache_dir().join("lazy-pulumi");
            std::fs::create_dir_all(&cache_dir).ok();
            cache_dir.join("app.log")
        })
        .unwrap_or_else(|| PathBuf::from("/tmp/lazy-pulumi.log"));

    // Store the path globally
    let _ = LOG_FILE_PATH.set(log_path.clone());

    // Create/truncate the log file
    let log_file = File::create(&log_path)?;

    // Set up tracing to write to file
    let file_layer = tracing_subscriber::fmt::layer()
        .with_writer(log_file)
        .with_ansi(false)
        .with_target(false);

    let env_filter = tracing_subscriber::EnvFilter::from_default_env()
        .add_directive(tracing::Level::INFO.into());

    tracing_subscriber::registry()
        .with(env_filter)
        .with(file_layer)
        .init();

    tracing::info!("Lazy Pulumi started - logging to {:?}", log_path);

    Ok(())
}

/// Read the last N lines from the log file
pub fn read_log_lines(max_lines: usize) -> Vec<String> {
    let log_path = log_file_path();

    let file = match OpenOptions::new().read(true).open(&log_path) {
        Ok(f) => f,
        Err(_) => return vec!["Log file not found".to_string()],
    };

    let reader = BufReader::new(file);
    let all_lines: Vec<String> = reader.lines().filter_map(|l| l.ok()).collect();

    // Return last max_lines
    if all_lines.len() > max_lines {
        all_lines[all_lines.len() - max_lines..].to_vec()
    } else {
        all_lines
    }
}

/// Read all lines from the log file with an optional tail
pub fn read_log_tail(tail_lines: Option<usize>) -> Vec<String> {
    match tail_lines {
        Some(n) => read_log_lines(n),
        None => {
            let log_path = log_file_path();
            let file = match OpenOptions::new().read(true).open(&log_path) {
                Ok(f) => f,
                Err(_) => return vec!["Log file not found".to_string()],
            };
            BufReader::new(file).lines().filter_map(|l| l.ok()).collect()
        }
    }
}
