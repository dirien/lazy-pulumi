//! Configuration management
//!
//! Handles persisting user preferences like splash screen settings.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// User configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Whether to show the splash screen on startup
    #[serde(default = "default_show_splash")]
    pub show_splash: bool,
}

fn default_show_splash() -> bool {
    true
}

impl Default for Config {
    fn default() -> Self {
        Self { show_splash: true }
    }
}

impl Config {
    /// Get the config file path
    pub fn config_path() -> PathBuf {
        directories::BaseDirs::new()
            .map(|dirs| {
                let config_dir = dirs.config_dir().join("lazy-pulumi");
                fs::create_dir_all(&config_dir).ok();
                config_dir.join("config.json")
            })
            .unwrap_or_else(|| PathBuf::from("/tmp/lazy-pulumi-config.json"))
    }

    /// Load configuration from file
    pub fn load() -> Self {
        let path = Self::config_path();

        if path.exists() {
            match fs::read_to_string(&path) {
                Ok(contents) => match serde_json::from_str(&contents) {
                    Ok(config) => return config,
                    Err(e) => {
                        log::warn!("Failed to parse config: {}", e);
                    }
                },
                Err(e) => {
                    log::warn!("Failed to read config: {}", e);
                }
            }
        }

        // Return default config
        Self::default()
    }

    /// Save configuration to file
    pub fn save(&self) {
        let path = Self::config_path();

        match serde_json::to_string_pretty(self) {
            Ok(contents) => {
                if let Err(e) = fs::write(&path, contents) {
                    log::warn!("Failed to save config: {}", e);
                }
            }
            Err(e) => {
                log::warn!("Failed to serialize config: {}", e);
            }
        }
    }
}
