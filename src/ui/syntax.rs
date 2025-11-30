//! Syntax highlighting utilities using syntect

use once_cell::sync::Lazy;
use ratatui::style::Style as RatatuiStyle;
use ratatui::text::{Line, Span};
use syntect::easy::HighlightLines;
use syntect::highlighting::{Style as SyntectStyle, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

/// Lazy-loaded syntax set with default syntaxes
static SYNTAX_SET: Lazy<SyntaxSet> = Lazy::new(SyntaxSet::load_defaults_newlines);

/// Lazy-loaded theme set with default themes
static THEME_SET: Lazy<ThemeSet> = Lazy::new(ThemeSet::load_defaults);

/// Convert syntect style to ratatui style with owned content
fn syntect_to_ratatui_span(style: SyntectStyle, content: &str) -> Span<'static> {
    let fg = ratatui::style::Color::Rgb(
        style.foreground.r,
        style.foreground.g,
        style.foreground.b,
    );

    Span::styled(
        content.to_string(),
        RatatuiStyle::default().fg(fg),
    )
}

/// Highlight YAML content and return ratatui Lines
pub fn highlight_yaml(content: &str) -> Vec<Line<'static>> {
    let syntax = SYNTAX_SET
        .find_syntax_by_extension("yaml")
        .or_else(|| SYNTAX_SET.find_syntax_by_extension("yml"))
        .unwrap_or_else(|| SYNTAX_SET.find_syntax_plain_text());

    // Use a dark theme that works well in terminals
    let theme = THEME_SET
        .themes
        .get("base16-ocean.dark")
        .or_else(|| THEME_SET.themes.get("base16-eighties.dark"))
        .unwrap_or_else(|| THEME_SET.themes.values().next().unwrap());

    let mut highlighter = HighlightLines::new(syntax, theme);
    let mut lines = Vec::new();

    for line in LinesWithEndings::from(content) {
        match highlighter.highlight_line(line, &SYNTAX_SET) {
            Ok(highlighted) => {
                let spans: Vec<Span<'static>> = highlighted
                    .into_iter()
                    .map(|(style, text)| syntect_to_ratatui_span(style, text))
                    .collect();
                lines.push(Line::from(spans));
            }
            Err(_) => {
                // Fallback to plain text if highlighting fails
                lines.push(Line::from(line.trim_end().to_string()));
            }
        }
    }

    lines
}

/// Highlight JSON content and return ratatui Lines
pub fn highlight_json(content: &str) -> Vec<Line<'static>> {
    let syntax = SYNTAX_SET
        .find_syntax_by_extension("json")
        .unwrap_or_else(|| SYNTAX_SET.find_syntax_plain_text());

    let theme = THEME_SET
        .themes
        .get("base16-ocean.dark")
        .or_else(|| THEME_SET.themes.get("base16-eighties.dark"))
        .unwrap_or_else(|| THEME_SET.themes.values().next().unwrap());

    let mut highlighter = HighlightLines::new(syntax, theme);
    let mut lines = Vec::new();

    for line in LinesWithEndings::from(content) {
        match highlighter.highlight_line(line, &SYNTAX_SET) {
            Ok(highlighted) => {
                let spans: Vec<Span<'static>> = highlighted
                    .into_iter()
                    .map(|(style, text)| syntect_to_ratatui_span(style, text))
                    .collect();
                lines.push(Line::from(spans));
            }
            Err(_) => {
                lines.push(Line::from(line.trim_end().to_string()));
            }
        }
    }

    lines
}
