//! Markdown rendering utilities for TUI
//!
//! Provides functions for parsing and rendering markdown content
//! with styled text for Ratatui widgets.

use ratatui::{
    prelude::*,
    style::Modifier,
    text::{Line, Span},
};

use crate::theme::{symbols, Theme};

/// Parse markdown content in a single line into styled spans (returns owned data)
#[allow(clippy::while_let_on_iterator)]
pub fn parse_markdown_line(text: &str, theme: &Theme) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let mut chars = text.char_indices().peekable();
    let mut current_text = String::new();

    while let Some((_i, c)) = chars.next() {
        match c {
            // Bold: **text** or __text__
            '*' | '_' => {
                if let Some(&(_, next_c)) = chars.peek() {
                    if next_c == c {
                        // Double marker - bold
                        chars.next(); // consume second marker
                        if !current_text.is_empty() {
                            spans.push(Span::styled(
                                std::mem::take(&mut current_text),
                                theme.text(),
                            ));
                        }
                        // Find closing **
                        let mut bold_text = String::new();
                        let mut found_end = false;
                        while let Some((_, bc)) = chars.next() {
                            if bc == c {
                                if let Some(&(_, nc)) = chars.peek() {
                                    if nc == c {
                                        chars.next();
                                        found_end = true;
                                        break;
                                    }
                                }
                            }
                            bold_text.push(bc);
                        }
                        if found_end && !bold_text.is_empty() {
                            spans.push(Span::styled(
                                bold_text,
                                theme.text().add_modifier(Modifier::BOLD),
                            ));
                        } else {
                            current_text.push(c);
                            current_text.push(c);
                            current_text.push_str(&bold_text);
                        }
                    } else {
                        // Single marker - italic
                        if !current_text.is_empty() {
                            spans.push(Span::styled(
                                std::mem::take(&mut current_text),
                                theme.text(),
                            ));
                        }
                        let mut italic_text = String::new();
                        let mut found_end = false;
                        while let Some((_, ic)) = chars.next() {
                            if ic == c {
                                found_end = true;
                                break;
                            }
                            italic_text.push(ic);
                        }
                        if found_end && !italic_text.is_empty() {
                            spans.push(Span::styled(
                                italic_text,
                                theme.text().add_modifier(Modifier::ITALIC),
                            ));
                        } else {
                            current_text.push(c);
                            current_text.push_str(&italic_text);
                        }
                    }
                } else {
                    current_text.push(c);
                }
            }
            // Inline code: `code`
            '`' => {
                // Check for code block (```)
                let mut backtick_count = 1;
                while let Some(&(_, '`')) = chars.peek() {
                    chars.next();
                    backtick_count += 1;
                }

                if backtick_count >= 3 {
                    // Code block start - just add as-is for now (handled at line level)
                    current_text.push_str(&"`".repeat(backtick_count));
                } else {
                    // Inline code
                    if !current_text.is_empty() {
                        spans.push(Span::styled(
                            std::mem::take(&mut current_text),
                            theme.text(),
                        ));
                    }
                    let mut code_text = String::new();
                    let mut found_end = false;
                    while let Some((_, cc)) = chars.next() {
                        if cc == '`' {
                            found_end = true;
                            break;
                        }
                        code_text.push(cc);
                    }
                    if found_end {
                        spans.push(Span::styled(
                            format!(" {} ", code_text),
                            Style::default().fg(theme.accent).bg(theme.bg_light),
                        ));
                    } else {
                        current_text.push('`');
                        current_text.push_str(&code_text);
                    }
                }
            }
            _ => {
                current_text.push(c);
            }
        }
    }

    if !current_text.is_empty() {
        spans.push(Span::styled(current_text, theme.text()));
    }

    if spans.is_empty() {
        spans.push(Span::raw(""));
    }

    spans
}

/// Render markdown content as styled lines (returns owned data)
pub fn render_markdown_content(content: &str, theme: &Theme, indent: &str) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let mut in_code_block = false;
    let mut code_lang = String::new();
    let mut code_lines: Vec<String> = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();

        // Check for code block markers
        if trimmed.starts_with("```") {
            if in_code_block {
                // End of code block - render accumulated code
                if !code_lines.is_empty() {
                    // Add language label if present
                    if !code_lang.is_empty() {
                        lines.push(Line::from(vec![
                            Span::styled(indent.to_string(), theme.text()),
                            Span::styled(format!("─── {} ", code_lang), theme.text_muted()),
                            Span::styled("───────────────────".to_string(), theme.text_muted()),
                        ]));
                    } else {
                        lines.push(Line::from(vec![
                            Span::styled(indent.to_string(), theme.text()),
                            Span::styled(
                                "─────────────────────────".to_string(),
                                theme.text_muted(),
                            ),
                        ]));
                    }
                    for code_line in code_lines.drain(..) {
                        lines.push(Line::from(vec![
                            Span::styled(indent.to_string(), theme.text()),
                            Span::styled(
                                format!("  {}", code_line),
                                Style::default().fg(theme.accent).bg(theme.bg_medium),
                            ),
                        ]));
                    }
                    lines.push(Line::from(vec![
                        Span::styled(indent.to_string(), theme.text()),
                        Span::styled("─────────────────────────".to_string(), theme.text_muted()),
                    ]));
                }
                in_code_block = false;
                code_lang.clear();
            } else {
                // Start of code block
                in_code_block = true;
                code_lang = trimmed.trim_start_matches('`').to_string();
            }
            continue;
        }

        if in_code_block {
            code_lines.push(line.to_string());
            continue;
        }

        // Handle headers
        if trimmed.starts_with("### ") {
            lines.push(Line::from(vec![
                Span::styled(indent.to_string(), theme.text()),
                Span::styled(
                    trimmed.trim_start_matches("### ").to_string(),
                    theme
                        .text()
                        .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
                ),
            ]));
        } else if trimmed.starts_with("## ") {
            lines.push(Line::from(vec![
                Span::styled(indent.to_string(), theme.text()),
                Span::styled(
                    trimmed.trim_start_matches("## ").to_string(),
                    theme.primary().add_modifier(Modifier::BOLD),
                ),
            ]));
        } else if trimmed.starts_with("# ") {
            lines.push(Line::from(vec![
                Span::styled(indent.to_string(), theme.text()),
                Span::styled(
                    trimmed.trim_start_matches("# ").to_string(),
                    theme
                        .primary()
                        .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
                ),
            ]));
        }
        // Handle list items
        else if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
            let item_text = &trimmed[2..];
            let mut line_spans = vec![
                Span::styled(indent.to_string(), theme.text()),
                Span::styled(format!("{} ", symbols::BULLET), theme.accent()),
            ];
            line_spans.extend(parse_markdown_line(item_text, theme));
            lines.push(Line::from(line_spans));
        }
        // Handle numbered lists
        else if trimmed
            .chars()
            .next()
            .map(|c| c.is_ascii_digit())
            .unwrap_or(false)
            && trimmed.contains(". ")
        {
            if let Some(dot_pos) = trimmed.find(". ") {
                let num = &trimmed[..dot_pos];
                let item_text = &trimmed[dot_pos + 2..];
                let mut line_spans = vec![
                    Span::styled(indent.to_string(), theme.text()),
                    Span::styled(format!("{}. ", num), theme.accent()),
                ];
                line_spans.extend(parse_markdown_line(item_text, theme));
                lines.push(Line::from(line_spans));
            } else {
                let mut line_spans = vec![Span::styled(indent.to_string(), theme.text())];
                line_spans.extend(parse_markdown_line(line, theme));
                lines.push(Line::from(line_spans));
            }
        }
        // Regular text with markdown parsing
        else {
            let mut line_spans = vec![Span::styled(indent.to_string(), theme.text())];
            line_spans.extend(parse_markdown_line(line, theme));
            lines.push(Line::from(line_spans));
        }
    }

    // Handle unclosed code block
    if in_code_block && !code_lines.is_empty() {
        lines.push(Line::from(vec![
            Span::styled(indent.to_string(), theme.text()),
            Span::styled("─────────────────────────".to_string(), theme.text_muted()),
        ]));
        for code_line in code_lines {
            lines.push(Line::from(vec![
                Span::styled(indent.to_string(), theme.text()),
                Span::styled(
                    format!("  {}", code_line),
                    Style::default().fg(theme.accent).bg(theme.bg_medium),
                ),
            ]));
        }
    }

    lines
}
