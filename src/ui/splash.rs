//! Splash screen rendering
//!
//! Displays the Pulumi logo with "Lazy Pulumi" title and startup checklist.
//! The logo is drawn as pixel art with half-block characters (two square pixels
//! per terminal cell) and scales to fit the terminal while keeping its aspect
//! ratio. When there is too little room for a legible wordmark, only the cube
//! mark is drawn.

use image::{imageops, RgbaImage};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    prelude::*,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};
use std::sync::OnceLock;

use crate::startup::{CheckStatus, StartupChecks};
use crate::theme::Theme;

/// Application version - uses GoReleaser injected version if available, otherwise falls back to Cargo.toml version
const VERSION: &str = match option_env!("APP_VERSION") {
    Some(v) => v,
    None => env!("CARGO_PKG_VERSION"),
};

/// Width of the cube mark in the source image (the wordmark starts at x=145)
const MARK_WIDTH: u32 = 100;

/// Below this pixel height the wordmark strokes are unreadable, so only the mark is drawn
const MIN_WORDMARK_PX_HEIGHT: u32 = 20;

/// Below this pixel height even the mark is unrecognizable, so no logo is drawn
const MIN_MARK_PX_HEIGHT: u32 = 6;

/// Upper bound for the logo height in terminal rows
const MAX_LOGO_ROWS: u16 = 14;

/// Cached original image
static IMAGE_CACHE: OnceLock<RgbaImage> = OnceLock::new();

/// Load the original image (cached)
fn get_image() -> &'static RgbaImage {
    IMAGE_CACHE.get_or_init(|| {
        let image_bytes = include_bytes!("../../assets/logo-on-black.png");
        image::load_from_memory(image_bytes)
            .expect("Failed to load embedded Pulumi logo")
            .to_rgba8()
    })
}

/// Pixel grid; `None` is a transparent pixel
type PixelGrid = Vec<Vec<Option<Color>>>;

/// Which part of the logo to draw and at what size (in pixels)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LogoFit {
    /// Columns of the source image to use (full image or mark only)
    crop_width: u32,
    width: u32,
    height: u32,
}

/// Pick the largest logo variant that fits in `max_cols` x `max_rows` cells.
///
/// Each cell holds two vertically stacked pixels, so pixels are square.
fn fit_logo(src_width: u32, src_height: u32, max_cols: u16, max_rows: u16) -> Option<LogoFit> {
    let max_px_height = 2 * u32::from(max_rows.min(MAX_LOGO_ROWS));
    let max_px_width = u32::from(max_cols);

    let fit = |crop_width: u32| {
        let height = max_px_height.min(max_px_width * src_height / crop_width);
        let width = (crop_width * height).div_ceil(src_height).min(max_px_width);
        LogoFit {
            crop_width,
            width,
            height,
        }
    };

    let full = fit(src_width);
    if full.height >= MIN_WORDMARK_PX_HEIGHT {
        return Some(full);
    }
    let mark = fit(MARK_WIDTH.min(src_width));
    (mark.height >= MIN_MARK_PX_HEIGHT).then_some(mark)
}

/// Downscale with area averaging in premultiplied alpha.
///
/// Every target pixel averages the source pixels it covers, so thin strokes and
/// anti-aliased edges keep their real color instead of ringing (Lanczos) or
/// fading into the background. A pixel is drawn when at least half of its area
/// is covered.
fn downsample(img: &RgbaImage, width: u32, height: u32) -> PixelGrid {
    let (src_width, src_height) = img.dimensions();
    let span = |i: u32, target: u32, source: u32| {
        let start = i * source / target;
        let end = ((i + 1) * source / target).max(start + 1).min(source);
        start..end
    };

    (0..height)
        .map(|ty| {
            let ys = span(ty, height, src_height);
            (0..width)
                .map(|tx| {
                    let xs = span(tx, width, src_width);
                    let (mut r, mut g, mut b, mut a) = (0u64, 0u64, 0u64, 0u64);
                    for y in ys.clone() {
                        for x in xs.clone() {
                            let [pr, pg, pb, pa] = img.get_pixel(x, y).0;
                            let pa = u64::from(pa);
                            r += u64::from(pr) * pa;
                            g += u64::from(pg) * pa;
                            b += u64::from(pb) * pa;
                            a += pa;
                        }
                    }
                    let area = (xs.len() * ys.len()) as u64;
                    if a == 0 || a * 2 < area * 255 {
                        return None;
                    }
                    Some(Color::Rgb((r / a) as u8, (g / a) as u8, (b / a) as u8))
                })
                .collect()
        })
        .collect()
}

/// Render the logo pixels for the available area, or `None` if it doesn't fit
fn logo_pixels(max_cols: u16, max_rows: u16) -> Option<PixelGrid> {
    let img = get_image();
    let fit = fit_logo(img.width(), img.height(), max_cols, max_rows)?;
    if fit.crop_width == img.width() {
        Some(downsample(img, fit.width, fit.height))
    } else {
        let mark = imageops::crop_imm(img, 0, 0, fit.crop_width, img.height()).to_image();
        Some(downsample(&mark, fit.width, fit.height))
    }
}

/// Render the splash screen with startup checklist
pub fn render_splash(
    frame: &mut Frame,
    theme: &Theme,
    spinner_char: &str,
    dont_show_again: bool,
    checks: &StartupChecks,
) {
    let area = frame.area();

    // Fixed-height content below the logo
    let title_height: u16 = 1;
    let version_height: u16 = 1;
    let checklist_height: u16 = 4; // 2 checks with status
    let action_height: u16 = 2;
    let checkbox_height: u16 = 3;
    let content_height =
        title_height + version_height + checklist_height + action_height + checkbox_height;

    // Gaps after: logo, title, version, checklist, action hint.
    // Tighten them on short terminals so the logo keeps some room.
    let roomy_gaps = [2, 1, 2, 2, 2];
    let compact_gaps = [1, 0, 1, 1, 1];
    let roomy_height = content_height + roomy_gaps.iter().sum::<u16>();
    let gaps = if area.height >= roomy_height + MAX_LOGO_ROWS / 2 {
        roomy_gaps
    } else {
        compact_gaps
    };
    let reserved_height = content_height + gaps.iter().sum::<u16>();

    let logo = logo_pixels(
        area.width.saturating_sub(4), // Leave some margin
        area.height.saturating_sub(reserved_height),
    );
    let logo_lines = logo.as_deref().map(pixels_to_lines).unwrap_or_default();
    let logo_height = logo_lines.len() as u16;
    let logo_gap = if logo_height > 0 { gaps[0] } else { 0 };

    let total_content_height = logo_height + logo_gap + reserved_height - gaps[0];

    // Center everything vertically
    let vertical_padding = area.height.saturating_sub(total_content_height) / 2;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(vertical_padding),
            Constraint::Length(logo_height),
            Constraint::Length(logo_gap),
            Constraint::Length(title_height),
            Constraint::Length(gaps[1]),
            Constraint::Length(version_height),
            Constraint::Length(gaps[2]),
            Constraint::Length(checklist_height),
            Constraint::Length(gaps[3]),
            Constraint::Length(action_height),
            Constraint::Length(gaps[4]),
            Constraint::Length(checkbox_height),
            Constraint::Min(0),
        ])
        .split(area);

    // Render logo
    let logo_paragraph = Paragraph::new(logo_lines).alignment(Alignment::Center);

    frame.render_widget(logo_paragraph, chunks[1]);

    // Render title - use Pulumi Violet for brand consistency
    let title_line = Line::from(vec![Span::styled(
        "Lazy Pulumi",
        Style::default()
            .fg(theme.colors.violet)
            .add_modifier(Modifier::BOLD),
    )]);

    let title_paragraph = Paragraph::new(title_line).alignment(Alignment::Center);

    frame.render_widget(title_paragraph, chunks[3]);

    // Render version
    let version_line = Line::from(vec![Span::styled(
        format!("v{}", VERSION.trim_start_matches('v')),
        Style::default().fg(theme.text_muted),
    )]);

    let version_paragraph = Paragraph::new(version_line).alignment(Alignment::Center);

    frame.render_widget(version_paragraph, chunks[5]);

    // Render startup checklist
    let checklist_lines = render_checklist(theme, spinner_char, checks);
    let checklist_paragraph = Paragraph::new(checklist_lines).alignment(Alignment::Center);

    frame.render_widget(checklist_paragraph, chunks[7]);

    // Render action hint (press enter to continue, or error message)
    let action_lines = render_action_hint(theme, checks);
    let action_paragraph = Paragraph::new(action_lines).alignment(Alignment::Center);

    frame.render_widget(action_paragraph, chunks[9]);

    // Render checkbox (only if checks passed)
    if checks.all_passed() {
        let checkbox_icon = if dont_show_again { "[x]" } else { "[ ]" };
        let checkbox_line = Line::from(vec![
            Span::styled(
                checkbox_icon,
                Style::default()
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " Don't show this again",
                Style::default().fg(theme.text_secondary),
            ),
        ]);

        let hint_line = Line::from(vec![
            Span::styled("Press ", Style::default().fg(theme.text_muted)),
            Span::styled(
                "Space",
                Style::default()
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" to toggle", Style::default().fg(theme.text_muted)),
        ]);

        let checkbox_paragraph =
            Paragraph::new(vec![checkbox_line, hint_line]).alignment(Alignment::Center);

        frame.render_widget(checkbox_paragraph, chunks[11]);
    }
}

/// Render the startup checklist items
fn render_checklist(
    theme: &Theme,
    spinner_char: &str,
    checks: &StartupChecks,
) -> Vec<Line<'static>> {
    vec![
        render_check_line(
            theme,
            spinner_char,
            checks.token_check.name,
            &checks.token_check.status,
        ),
        render_check_line(
            theme,
            spinner_char,
            checks.cli_check.name,
            &checks.cli_check.status,
        ),
    ]
}

/// Render a single check line
fn render_check_line(
    theme: &Theme,
    spinner_char: &str,
    name: &str,
    status: &CheckStatus,
) -> Line<'static> {
    let (icon, icon_style, detail) = match status {
        CheckStatus::Pending => (
            "○".to_string(),
            Style::default().fg(theme.text_muted),
            "Waiting...".to_string(),
        ),
        CheckStatus::Running => (
            spinner_char.to_string(),
            Style::default().fg(theme.colors.yellow),
            "Checking...".to_string(),
        ),
        CheckStatus::Passed(msg) => (
            "✓".to_string(),
            Style::default().fg(theme.success),
            msg.clone(),
        ),
        CheckStatus::Failed(msg) => (
            "✗".to_string(),
            Style::default().fg(theme.colors.salmon),
            msg.clone(),
        ),
    };

    let detail_style = match status {
        CheckStatus::Failed(_) => Style::default().fg(theme.colors.salmon),
        CheckStatus::Passed(_) => Style::default().fg(theme.text_muted),
        _ => Style::default().fg(theme.text_muted),
    };

    Line::from(vec![
        Span::styled(format!("{} ", icon), icon_style),
        Span::styled(
            format!("{}: ", name),
            Style::default().fg(theme.text_secondary),
        ),
        Span::styled(detail, detail_style),
    ])
}

/// Render the action hint based on check status
fn render_action_hint(theme: &Theme, checks: &StartupChecks) -> Vec<Line<'static>> {
    if checks.any_running() || !checks.all_complete() {
        // Still running checks
        vec![Line::from(vec![Span::styled(
            "Running startup checks...",
            Style::default().fg(theme.text_muted),
        )])]
    } else if checks.any_failed() {
        // Checks failed - only allow quitting
        vec![
            Line::from(vec![Span::styled(
                "Startup checks failed",
                Style::default()
                    .fg(theme.colors.salmon)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![
                Span::styled("Press ", Style::default().fg(theme.text_muted)),
                Span::styled(
                    "q",
                    Style::default()
                        .fg(theme.colors.salmon)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" to quit", Style::default().fg(theme.text_muted)),
            ]),
        ]
    } else {
        // All checks passed
        vec![Line::from(vec![
            Span::styled("Press ", Style::default().fg(theme.text_muted)),
            Span::styled(
                "Enter",
                Style::default()
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" to continue", Style::default().fg(theme.text_muted)),
        ])]
    }
}

/// Convert a pixel grid to lines using half-block characters.
///
/// Each cell shows two pixels: `▀` with the top pixel as foreground and the
/// bottom pixel as background. Runs of identical cells are merged into one span.
fn pixels_to_lines(pixels: &[Vec<Option<Color>>]) -> Vec<Line<'static>> {
    pixels
        .chunks(2)
        .map(|pair| {
            let top = &pair[0];
            let bottom = pair.get(1);
            let mut spans: Vec<Span<'static>> = Vec::new();
            let mut run = String::new();
            let mut run_style = Style::default();

            for (x, &upper) in top.iter().enumerate() {
                let lower = bottom.and_then(|row| row.get(x).copied().flatten());
                let (ch, style) = match (upper, lower) {
                    (None, None) => (' ', Style::default()),
                    (Some(t), None) => ('▀', Style::default().fg(t)),
                    (None, Some(b)) => ('▄', Style::default().fg(b)),
                    (Some(t), Some(b)) => ('▀', Style::default().fg(t).bg(b)),
                };
                if style != run_style && !run.is_empty() {
                    spans.push(Span::styled(std::mem::take(&mut run), run_style));
                }
                run_style = style;
                run.push(ch);
            }
            if !run.is_empty() {
                spans.push(Span::styled(run, run_style));
            }
            Line::from(spans)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    const RED: Color = Color::Rgb(255, 0, 0);
    const BLUE: Color = Color::Rgb(0, 0, 255);

    #[test]
    fn fit_logo_uses_full_logo_when_tall_enough() {
        let fit = fit_logo(425, 106, 200, 40).expect("logo should fit");
        assert_eq!(fit.crop_width, 425);
        assert_eq!(fit.height, 2 * u32::from(MAX_LOGO_ROWS));
        assert!(fit.width <= 200);
    }

    #[test]
    fn fit_logo_is_width_limited_on_narrow_terminals() {
        let fit = fit_logo(425, 106, 100, 40).expect("logo should fit");
        assert_eq!(fit.crop_width, 425);
        assert!(fit.width <= 100);
        assert_eq!(fit.height, 100 * 106 / 425);
    }

    #[test]
    fn fit_logo_falls_back_to_mark_on_short_terminals() {
        let fit = fit_logo(425, 106, 120, 8).expect("mark should fit");
        assert_eq!(fit.crop_width, MARK_WIDTH);
        assert_eq!(fit.height, 16);
    }

    #[test]
    fn fit_logo_hides_logo_when_too_small() {
        assert_eq!(fit_logo(425, 106, 120, 2), None);
        assert_eq!(fit_logo(425, 106, 3, 20), None);
    }

    #[test]
    fn downsample_uses_coverage_and_keeps_color() {
        // 4x2 image: left half opaque red, right half transparent
        let img = RgbaImage::from_fn(4, 2, |x, _| {
            if x < 2 {
                Rgba([255, 0, 0, 255])
            } else {
                Rgba([0, 0, 0, 0])
            }
        });
        assert_eq!(downsample(&img, 2, 1), vec![vec![Some(RED), None]]);
    }

    #[test]
    fn downsample_ignores_color_of_transparent_pixels() {
        // Three opaque red pixels and one transparent white pixel stay pure red
        let img = RgbaImage::from_fn(2, 2, |x, y| {
            if x == 1 && y == 1 {
                Rgba([255, 255, 255, 0])
            } else {
                Rgba([255, 0, 0, 255])
            }
        });
        assert_eq!(downsample(&img, 1, 1), vec![vec![Some(RED)]]);
    }

    #[test]
    fn downsample_drops_mostly_transparent_pixels() {
        let img = RgbaImage::from_fn(2, 2, |x, y| {
            if x == 0 && y == 0 {
                Rgba([255, 0, 0, 255])
            } else {
                Rgba([0, 0, 0, 0])
            }
        });
        assert_eq!(downsample(&img, 1, 1), vec![vec![None]]);
    }

    #[test]
    fn pixels_to_lines_packs_two_rows_per_line() {
        let pixels = vec![
            vec![Some(RED), None, Some(RED), None],
            vec![Some(BLUE), Some(BLUE), None, None],
            vec![Some(RED), None, None, None],
        ];
        let lines = pixels_to_lines(&pixels);
        assert_eq!(lines.len(), 2);

        let first: Vec<(String, Style)> = lines[0]
            .spans
            .iter()
            .map(|s| (s.content.to_string(), s.style))
            .collect();
        assert_eq!(
            first,
            vec![
                ("▀".to_string(), Style::default().fg(RED).bg(BLUE)),
                ("▄".to_string(), Style::default().fg(BLUE)),
                ("▀".to_string(), Style::default().fg(RED)),
                (" ".to_string(), Style::default()),
            ]
        );
        // Odd final row renders as top halves only
        assert_eq!(lines[1].spans[0].content, "▀");
    }

    #[test]
    fn pixels_to_lines_merges_identical_cells() {
        let pixels = vec![vec![Some(RED); 5], vec![Some(RED); 5]];
        let lines = pixels_to_lines(&pixels);
        assert_eq!(lines[0].spans.len(), 1);
        assert_eq!(lines[0].spans[0].content, "▀▀▀▀▀");
    }

    #[test]
    fn logo_renders_at_common_sizes() {
        for (cols, rows) in [(80, 4), (120, 8), (200, 30)] {
            let pixels = logo_pixels(cols, rows).expect("logo should render");
            assert!(pixels.len() <= 2 * usize::from(rows));
            assert!(pixels.iter().all(|row| row.len() <= usize::from(cols)));
            assert!(pixels.iter().flatten().any(Option::is_some));
        }
    }
}
