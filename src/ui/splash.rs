//! Splash screen rendering
//!
//! Displays the Pulumi logo with "Lazy Pulumi" title.
//! The logo scales to fit the terminal while maintaining aspect ratio.

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

/// Cached original image
static IMAGE_CACHE: OnceLock<DynamicImage> = OnceLock::new();

/// Load the original image (cached)
fn get_image() -> &'static DynamicImage {
    IMAGE_CACHE.get_or_init(|| {
        let image_bytes = include_bytes!("../../assets/logo-on-black.png");
        image::load_from_memory(image_bytes).expect("Failed to load embedded Pulumi logo")
    })
}

/// Convert image to pixel color grid at specified dimensions
fn image_to_pixels(img: &DynamicImage, target_width: u32, target_height: u32) -> Vec<Vec<Option<Color>>> {
    // Use resize_exact to get exact dimensions we want
    let resized = img.resize_exact(
        target_width,
        target_height,
        image::imageops::FilterType::Lanczos3,
    );

    let (actual_width, actual_height) = resized.dimensions();
    let mut pixels = Vec::with_capacity(actual_height as usize);

    for y in 0..actual_height {
        let mut row = Vec::with_capacity(actual_width as usize);
        for x in 0..actual_width {
            let pixel = resized.get_pixel(x, y);
            let color = rgba_to_color(pixel);
            row.push(color);
        }
        pixels.push(row);
    }

    pixels
}

/// Convert RGBA pixel to ratatui Color, returns None for transparent or black pixels
fn rgba_to_color(pixel: Rgba<u8>) -> Option<Color> {
    let [r, g, b, a] = pixel.0;

    // Skip transparent pixels
    if a < 128 {
        return None;
    }

    // Skip black/near-black pixels (the background)
    if r < 20 && g < 20 && b < 20 {
        return None;
    }

    Some(Color::Rgb(r, g, b))
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
    let img = get_image();

    // Reserve space for title, version, loading, checkbox, and spacing
    let title_height: u16 = 1;
    let version_height: u16 = 1;
    let checkbox_height: u16 = 3;
    let loading_height: u16 = 2;
    let spacing: u16 = 8; // Total spacing between elements
    let reserved_height = title_height + version_height + checkbox_height + loading_height + spacing;

    // Calculate available space for the logo
    let available_height = area.height.saturating_sub(reserved_height);
    let available_width = area.width.saturating_sub(4); // Leave some margin

    // Get original image dimensions (425x106 - wide logo)
    let (orig_width, orig_height) = img.dimensions();
    let image_aspect = orig_width as f32 / orig_height as f32; // ~4:1

    // Terminal characters are typically about 2:1 height to width ratio
    // To maintain visual aspect ratio: visual_width / visual_height = image_aspect
    // Since terminal chars are 2x tall: pixel_width / pixel_height = image_aspect * 2

    let effective_aspect = image_aspect * 2.0;

    // Calculate dimensions to fit available space
    let max_height = available_height.min(25) as f32; // Cap height for this wide logo
    let max_width = available_width as f32;

    // Try using max height first
    let width_for_height = max_height * effective_aspect;

    let (final_width, final_height) = if width_for_height <= max_width {
        // Height is limiting
        (width_for_height as u32, max_height as u32)
    } else {
        // Width is limiting
        let h = max_width / effective_aspect;
        (max_width as u32, h as u32)
    };

    // Ensure minimum size
    let final_width = final_width.max(60);
    let final_height = final_height.max(8);

    // Generate pixel art at calculated dimensions
    let pixels = image_to_pixels(img, final_width, final_height);
    let pixel_height = pixels.len() as u16;

    let total_content_height = pixel_height + reserved_height;

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

    // Render logo
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
