//! Splash screen rendering
//!
//! Displays pixel art version of the Pulumi mascot with "Lazy Pulumi" title.

use image::{DynamicImage, GenericImageView, Rgba};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    prelude::*,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};
use std::sync::OnceLock;

use crate::theme::Theme;

/// Application version from Cargo.toml
const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Cached pixel art data
static PIXEL_ART_CACHE: OnceLock<PixelArtData> = OnceLock::new();

/// Pixel art data structure
struct PixelArtData {
    /// Full size pixel art (for larger terminals)
    full: Vec<Vec<Option<Color>>>,
    /// Small size pixel art (for smaller terminals)
    small: Vec<Vec<Option<Color>>>,
}

/// Load and convert image to pixel art
fn load_pixel_art() -> PixelArtData {
    // Embed the image at compile time
    let image_bytes = include_bytes!("../../assets/mascot.png");

    let img = image::load_from_memory(image_bytes)
        .expect("Failed to load embedded mascot image");

    // Create full size version with higher resolution for better quality
    // Using ~80 chars wide for good detail on modern terminals
    let full = image_to_pixels(&img, 80, 40);

    // Create small version for smaller terminals
    let small = image_to_pixels(&img, 40, 20);

    PixelArtData { full, small }
}

/// Convert image to pixel color grid
fn image_to_pixels(img: &DynamicImage, target_width: u32, target_height: u32) -> Vec<Vec<Option<Color>>> {
    let resized = img.resize_exact(
        target_width,
        target_height,
        image::imageops::FilterType::Lanczos3,
    );

    let mut pixels = Vec::with_capacity(target_height as usize);

    for y in 0..target_height {
        let mut row = Vec::with_capacity(target_width as usize);
        for x in 0..target_width {
            let pixel = resized.get_pixel(x, y);
            let color = rgba_to_color(pixel);
            row.push(color);
        }
        pixels.push(row);
    }

    pixels
}

/// Convert RGBA pixel to ratatui Color, returns None for transparent pixels
fn rgba_to_color(pixel: Rgba<u8>) -> Option<Color> {
    let [r, g, b, a] = pixel.0;

    // Skip transparent/nearly transparent pixels
    if a < 128 {
        return None;
    }

    Some(Color::Rgb(r, g, b))
}

/// Get or initialize pixel art cache
fn get_pixel_art() -> &'static PixelArtData {
    PIXEL_ART_CACHE.get_or_init(load_pixel_art)
}

/// Render the splash screen
pub fn render_splash(
    frame: &mut Frame,
    theme: &Theme,
    spinner_char: &str,
    dont_show_again: bool,
    is_loading: bool,
) {
    let area = frame.area();

    let pixel_art = get_pixel_art();

    // Determine if we have enough space for the full art
    // Full art is 80x40, small is 40x20
    let use_full_art = area.height >= 55 && area.width >= 90;

    let pixels = if use_full_art { &pixel_art.full } else { &pixel_art.small };
    let pixel_height = pixels.len() as u16;

    let title_height: u16 = 1;
    let version_height: u16 = 1;
    let checkbox_height: u16 = 3;
    let loading_height: u16 = 2;
    let total_content_height = pixel_height + title_height + version_height + checkbox_height + loading_height + 8;

    // Center everything vertically
    let vertical_padding = area.height.saturating_sub(total_content_height) / 2;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(vertical_padding),
            Constraint::Length(pixel_height),
            Constraint::Length(2), // spacing
            Constraint::Length(title_height),
            Constraint::Length(1), // spacing
            Constraint::Length(version_height),
            Constraint::Length(2), // spacing
            Constraint::Length(loading_height),
            Constraint::Length(2), // spacing
            Constraint::Length(checkbox_height),
            Constraint::Min(0),
        ])
        .split(area);

    // Render pixel art
    let pixel_lines: Vec<Line> = pixels
        .iter()
        .map(|row| pixels_to_line(row))
        .collect();

    let pixel_paragraph = Paragraph::new(pixel_lines)
        .alignment(Alignment::Center);

    frame.render_widget(pixel_paragraph, chunks[1]);

    // Render title
    let title_line = Line::from(vec![
        Span::styled(
            "Lazy Pulumi",
            Style::default().fg(theme.colors.cyan).add_modifier(Modifier::BOLD),
        ),
    ]);

    let title_paragraph = Paragraph::new(title_line)
        .alignment(Alignment::Center);

    frame.render_widget(title_paragraph, chunks[3]);

    // Render version
    let version_line = Line::from(vec![
        Span::styled(
            format!("v{}", VERSION),
            Style::default().fg(theme.text_muted),
        ),
    ]);

    let version_paragraph = Paragraph::new(version_line)
        .alignment(Alignment::Center);

    frame.render_widget(version_paragraph, chunks[5]);

    // Render loading indicator or "Press any key" message
    let loading_text = if is_loading {
        Line::from(vec![
            Span::styled(format!("{} ", spinner_char), theme.primary()),
            Span::styled("Loading...", Style::default().fg(theme.text_muted)),
        ])
    } else {
        Line::from(vec![
            Span::styled("Press ", Style::default().fg(theme.text_muted)),
            Span::styled("Enter", Style::default().fg(theme.primary).add_modifier(Modifier::BOLD)),
            Span::styled(" to continue", Style::default().fg(theme.text_muted)),
        ])
    };

    let loading_paragraph = Paragraph::new(loading_text)
        .alignment(Alignment::Center);

    frame.render_widget(loading_paragraph, chunks[7]);

    // Render checkbox
    let checkbox_icon = if dont_show_again { "[x]" } else { "[ ]" };
    let checkbox_line = Line::from(vec![
        Span::styled(
            checkbox_icon,
            Style::default().fg(theme.primary).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            " Don't show this again",
            Style::default().fg(theme.text_secondary),
        ),
    ]);

    let hint_line = Line::from(vec![
        Span::styled(
            "Press ",
            Style::default().fg(theme.text_muted),
        ),
        Span::styled(
            "Space",
            Style::default().fg(theme.primary).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            " to toggle",
            Style::default().fg(theme.text_muted),
        ),
    ]);

    let checkbox_paragraph = Paragraph::new(vec![checkbox_line, hint_line])
        .alignment(Alignment::Center);

    frame.render_widget(checkbox_paragraph, chunks[9]);
}

/// Convert a row of pixels to a Line with colored spans
fn pixels_to_line(row: &[Option<Color>]) -> Line<'static> {
    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut current_color: Option<Option<Color>> = None;
    let mut current_chars = String::new();

    for &pixel_color in row {
        if current_color != Some(pixel_color) {
            // Flush current buffer
            if !current_chars.is_empty() {
                let style = match current_color.flatten() {
                    Some(color) => Style::default().fg(color),
                    None => Style::default(),
                };
                spans.push(Span::styled(std::mem::take(&mut current_chars), style));
            }
            current_color = Some(pixel_color);
        }

        // Use block characters for pixels
        // █ (full block) for colored pixels, space for transparent
        let ch = if pixel_color.is_some() { '█' } else { ' ' };
        current_chars.push(ch);
    }

    // Flush remaining
    if !current_chars.is_empty() {
        let style = match current_color.flatten() {
            Some(color) => Style::default().fg(color),
            None => Style::default(),
        };
        spans.push(Span::styled(current_chars, style));
    }

    Line::from(spans)
}
