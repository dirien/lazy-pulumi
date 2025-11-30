//! Theme and styling for the Pulumi TUI
//!
//! A cohesive color palette inspired by Pulumi's brand colors
//! with a modern, easy-on-the-eyes design.

use ratatui::style::{Color, Modifier, Style};

/// Pulumi-inspired color palette
pub struct Theme {
    // Primary colors
    pub primary: Color,
    pub secondary: Color,
    pub accent: Color,

    // Background colors
    pub bg_dark: Color,
    pub bg_medium: Color,
    pub bg_light: Color,

    // Text colors
    pub text_primary: Color,
    pub text_secondary: Color,
    pub text_muted: Color,

    // Status colors
    pub success: Color,
    pub warning: Color,
    pub error: Color,
    pub info: Color,

    // Special colors
    pub highlight: Color,
    pub border: Color,
    pub border_focused: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            // Pulumi purple/violet primary
            primary: Color::Rgb(138, 94, 255),    // Pulumi purple
            secondary: Color::Rgb(99, 179, 237),  // Soft blue
            accent: Color::Rgb(246, 173, 85),     // Warm orange

            // Dark mode backgrounds
            bg_dark: Color::Rgb(22, 22, 30),      // Near black
            bg_medium: Color::Rgb(32, 32, 44),    // Dark gray
            bg_light: Color::Rgb(45, 45, 60),     // Medium dark

            // Text
            text_primary: Color::Rgb(240, 240, 250),
            text_secondary: Color::Rgb(180, 180, 200),
            text_muted: Color::Rgb(120, 120, 140),

            // Status
            success: Color::Rgb(72, 187, 120),    // Green
            warning: Color::Rgb(246, 173, 85),    // Orange
            error: Color::Rgb(245, 101, 101),     // Red
            info: Color::Rgb(99, 179, 237),       // Blue

            // Special
            highlight: Color::Rgb(138, 94, 255),
            border: Color::Rgb(60, 60, 80),
            border_focused: Color::Rgb(138, 94, 255),
        }
    }
}

impl Theme {
    /// Create a new theme
    pub fn new() -> Self {
        Self::default()
    }

    // ─────────────────────────────────────────────────────────────
    // Style builders
    // ─────────────────────────────────────────────────────────────

    /// Default text style
    pub fn text(&self) -> Style {
        Style::default().fg(self.text_primary)
    }

    /// Secondary text style
    pub fn text_secondary(&self) -> Style {
        Style::default().fg(self.text_secondary)
    }

    /// Muted text style
    pub fn text_muted(&self) -> Style {
        Style::default().fg(self.text_muted)
    }

    /// Primary accent style
    pub fn primary(&self) -> Style {
        Style::default().fg(self.primary)
    }

    /// Secondary accent style
    pub fn secondary(&self) -> Style {
        Style::default().fg(self.secondary)
    }

    /// Accent/highlight style
    pub fn accent(&self) -> Style {
        Style::default().fg(self.accent)
    }

    /// Title style
    pub fn title(&self) -> Style {
        Style::default()
            .fg(self.primary)
            .add_modifier(Modifier::BOLD)
    }

    /// Subtitle style
    pub fn subtitle(&self) -> Style {
        Style::default()
            .fg(self.text_secondary)
            .add_modifier(Modifier::ITALIC)
    }

    /// Block border style (unfocused)
    pub fn border(&self) -> Style {
        Style::default().fg(self.border)
    }

    /// Block border style (focused)
    pub fn border_focused(&self) -> Style {
        Style::default().fg(self.border_focused)
    }

    /// Selected item in a list
    pub fn selected(&self) -> Style {
        Style::default()
            .bg(self.bg_light)
            .fg(self.primary)
            .add_modifier(Modifier::BOLD)
    }

    /// Highlighted text
    pub fn highlight(&self) -> Style {
        Style::default()
            .fg(self.highlight)
            .add_modifier(Modifier::BOLD)
    }

    /// Success status
    pub fn success(&self) -> Style {
        Style::default().fg(self.success)
    }

    /// Warning status
    pub fn warning(&self) -> Style {
        Style::default().fg(self.warning)
    }

    /// Error status
    pub fn error(&self) -> Style {
        Style::default().fg(self.error)
    }

    /// Info status
    pub fn info(&self) -> Style {
        Style::default().fg(self.info)
    }

    /// Tab style (inactive)
    pub fn tab_inactive(&self) -> Style {
        Style::default().fg(self.text_muted)
    }

    /// Tab style (active)
    pub fn tab_active(&self) -> Style {
        Style::default()
            .fg(self.primary)
            .add_modifier(Modifier::BOLD)
    }

    /// Key hint style
    pub fn key_hint(&self) -> Style {
        Style::default()
            .fg(self.accent)
            .add_modifier(Modifier::BOLD)
    }

    /// Key description style
    pub fn key_desc(&self) -> Style {
        Style::default().fg(self.text_secondary)
    }

    /// Input field style
    pub fn input(&self) -> Style {
        Style::default().fg(self.text_primary)
    }

    /// Input cursor style
    pub fn cursor(&self) -> Style {
        Style::default()
            .bg(self.primary)
            .fg(self.bg_dark)
    }

    /// Status bar background
    #[allow(dead_code)]
    pub fn status_bar(&self) -> Style {
        Style::default().bg(self.bg_medium)
    }

    /// NEO message (from NEO)
    pub fn neo_message(&self) -> Style {
        Style::default().fg(self.secondary)
    }

    /// User message (from user)
    pub fn user_message(&self) -> Style {
        Style::default().fg(self.text_primary)
    }

    /// Get style for stack/resource status
    #[allow(dead_code)]
    pub fn status_style(&self, status: &str) -> Style {
        match status.to_lowercase().as_str() {
            "succeeded" | "success" | "active" | "running" => self.success(),
            "failed" | "error" => self.error(),
            "pending" | "in_progress" | "updating" => self.warning(),
            _ => self.text_secondary(),
        }
    }

    /// Sparkline colors for charts
    #[allow(dead_code)]
    pub fn sparkline(&self) -> Style {
        Style::default().fg(self.secondary)
    }

    /// Gauge filled style
    #[allow(dead_code)]
    pub fn gauge_filled(&self) -> Style {
        Style::default().fg(self.primary)
    }

    /// Gauge unfilled style
    #[allow(dead_code)]
    pub fn gauge_unfilled(&self) -> Style {
        Style::default().fg(self.bg_light)
    }
}

/// Box drawing characters for consistent UI
#[allow(dead_code)]
pub mod symbols {
    pub const VERTICAL: &str = "│";
    pub const HORIZONTAL: &str = "─";
    pub const TOP_LEFT: &str = "╭";
    pub const TOP_RIGHT: &str = "╮";
    pub const BOTTOM_LEFT: &str = "╰";
    pub const BOTTOM_RIGHT: &str = "╯";
    pub const CROSS: &str = "┼";
    pub const T_LEFT: &str = "├";
    pub const T_RIGHT: &str = "┤";
    pub const T_TOP: &str = "┬";
    pub const T_BOTTOM: &str = "┴";

    pub const BULLET: &str = "•";
    pub const ARROW_RIGHT: &str = "→";
    pub const ARROW_LEFT: &str = "←";
    pub const ARROW_UP: &str = "↑";
    pub const ARROW_DOWN: &str = "↓";
    pub const CHECK: &str = "✓";
    pub const CROSS_MARK: &str = "✗";
    pub const STAR: &str = "★";
    pub const DIAMOND: &str = "◆";

    pub const SPINNER: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
}
