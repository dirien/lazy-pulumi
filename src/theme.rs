//! Theme and styling for the Pulumi TUI
//!
//! Official Pulumi brand color palette for a polished, on-brand design.
//! Brand colors: Yellow, Salmon, Fuchsia, Purple, Violet, Blue

use ratatui::style::{Color, Modifier, Style};

/// Official Pulumi brand colors
pub mod brand {
    use ratatui::style::Color;

    /// Pulumi Yellow - #f7bf2a (RGB 247, 191, 42)
    pub const YELLOW: Color = Color::Rgb(247, 191, 42);

    /// Pulumi Salmon - #f26e7e (RGB 242, 110, 126)
    pub const SALMON: Color = Color::Rgb(242, 110, 126);

    /// Pulumi Fuchsia - #bd4c85 (RGB 189, 76, 133)
    pub const FUCHSIA: Color = Color::Rgb(189, 76, 133);

    /// Pulumi Purple - #8a3391 (RGB 138, 51, 145)
    pub const PURPLE: Color = Color::Rgb(138, 51, 145);

    /// Pulumi Violet - #805ac3 (RGB 128, 90, 195)
    pub const VIOLET: Color = Color::Rgb(128, 90, 195);

    /// Pulumi Blue - #4d5bd9 (RGB 77, 91, 217)
    pub const BLUE: Color = Color::Rgb(77, 91, 217);
}

/// Color palette for direct access to brand colors
#[allow(dead_code)]
pub struct Colors {
    pub yellow: Color,
    pub salmon: Color,
    pub fuchsia: Color,
    pub purple: Color,
    pub violet: Color,
    pub blue: Color,
}

impl Default for Colors {
    fn default() -> Self {
        Self {
            yellow: brand::YELLOW,
            salmon: brand::SALMON,
            fuchsia: brand::FUCHSIA,
            purple: brand::PURPLE,
            violet: brand::VIOLET,
            blue: brand::BLUE,
        }
    }
}

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

    // Direct color access
    pub colors: Colors,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            // Official Pulumi brand colors
            primary: brand::VIOLET,               // Pulumi Violet - main accent
            secondary: brand::BLUE,               // Pulumi Blue - secondary accent
            accent: brand::YELLOW,                // Pulumi Yellow - highlights/accents

            // Dark mode backgrounds (complement the brand colors)
            bg_dark: Color::Rgb(18, 18, 24),      // Deep dark with purple undertone
            bg_medium: Color::Rgb(28, 28, 38),    // Dark with subtle warmth
            bg_light: Color::Rgb(42, 42, 56),     // Medium dark

            // Text (light text for dark backgrounds)
            text_primary: Color::Rgb(245, 245, 252),
            text_secondary: Color::Rgb(185, 185, 205),
            text_muted: Color::Rgb(125, 125, 150),

            // Status colors (blend with brand palette)
            success: Color::Rgb(72, 187, 120),    // Green (kept for accessibility)
            warning: brand::YELLOW,               // Pulumi Yellow
            error: brand::SALMON,                 // Pulumi Salmon
            info: brand::BLUE,                    // Pulumi Blue

            // Special accents
            highlight: brand::FUCHSIA,            // Pulumi Fuchsia for highlights
            border: Color::Rgb(55, 55, 75),       // Muted border
            border_focused: brand::VIOLET,        // Pulumi Violet for focused borders

            // Direct color access
            colors: Colors::default(),
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

    /// Neo message (from Neo)
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

    // Additional icons for entity types
    pub const STACK: &str = "📚";
    pub const GEAR: &str = "⚙";
}
