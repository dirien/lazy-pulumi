//! Markdown rendering utilities for TUI
//!
//! Uses `tui-markdown` (pulldown-cmark based) for full markdown rendering
//! with syntax-highlighted code blocks, tables, blockquotes, and more.
//!
//! Since `tui-markdown` and `ratatui 0.30` both use `ratatui-core` types,
//! no conversion is needed — only ownership conversion for `'static` lifetimes.
//!
//! Tables are rendered by a custom fallback since `tui-markdown` 0.3.x does not
//! support them.

use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

use crate::theme::Theme;

/// Convert tui-markdown `Text` lines to owned `'static` lines.
fn to_owned_lines(text: ratatui::text::Text<'_>) -> Vec<Line<'static>> {
    text.lines
        .into_iter()
        .map(|line| {
            let spans: Vec<Span<'static>> = line
                .spans
                .into_iter()
                .map(|span| Span::styled(span.content.into_owned(), span.style))
                .collect();
            Line::from(spans)
        })
        .collect()
}

/// Render markdown content as styled lines.
///
/// Uses `tui_markdown` for proper AST-based markdown parsing with support for:
/// - Bold, italic, strikethrough, inline code
/// - Code blocks with syntax highlighting (via syntect)
/// - Headers (h1-h6)
/// - Bullet and numbered lists
/// - Blockquotes
/// - Links
/// - Horizontal rules
///
/// Tables are handled by a custom renderer since `tui-markdown` 0.3.x does not
/// support them yet.
pub fn render_markdown_content(content: &str, theme: &Theme, indent: &str) -> Vec<Line<'static>> {
    // Split content into segments: regular markdown vs. table blocks.
    // This lets us render tables ourselves while delegating everything else
    // to tui-markdown.
    let segments = split_tables(content);

    let mut lines: Vec<Line<'static>> = Vec::new();

    for segment in segments {
        match segment {
            Segment::Markdown(md) => {
                let text = tui_markdown::from_str(&md);
                lines.extend(to_owned_lines(text));
            }
            Segment::Table(raw) => {
                lines.extend(render_table(&raw, theme));
            }
        }
    }

    if indent.is_empty() {
        return lines;
    }

    // Prepend indent to each line
    let indent_owned = indent.to_string();
    lines
        .into_iter()
        .map(|line| {
            let mut spans = vec![Span::raw(indent_owned.clone())];
            spans.extend(line.spans);
            Line::from(spans)
        })
        .collect()
}

// ─── Table detection & splitting ────────────────────────────────────────────

enum Segment {
    Markdown(String),
    Table(String),
}

/// Returns true when the line looks like a markdown table row (has at least two
/// `|` characters, starting with `|` after optional leading whitespace).
fn is_table_line(line: &str) -> bool {
    let trimmed = line.trim_start();
    // A valid table row needs at least two pipes: `| cell |`
    if let Some(rest) = trimmed.strip_prefix('|') {
        rest.contains('|')
    } else {
        false
    }
}

/// Split markdown source into alternating Markdown / Table segments.
///
/// A "table block" is a contiguous run of lines that all look like table rows
/// (i.e. they start with `|`). Everything else is normal markdown.
fn split_tables(content: &str) -> Vec<Segment> {
    let mut segments: Vec<Segment> = Vec::new();
    let mut md_buf = String::new();
    let mut table_buf = String::new();

    for line in content.lines() {
        if is_table_line(line) {
            // Flush any accumulated markdown
            if !md_buf.is_empty() {
                segments.push(Segment::Markdown(std::mem::take(&mut md_buf)));
            }
            if !table_buf.is_empty() {
                table_buf.push('\n');
            }
            table_buf.push_str(line);
        } else {
            // Flush any accumulated table
            if !table_buf.is_empty() {
                segments.push(Segment::Table(std::mem::take(&mut table_buf)));
            }
            if !md_buf.is_empty() {
                md_buf.push('\n');
            }
            md_buf.push_str(line);
        }
    }

    // Flush remaining
    if !md_buf.is_empty() {
        segments.push(Segment::Markdown(md_buf));
    }
    if !table_buf.is_empty() {
        segments.push(Segment::Table(table_buf));
    }

    segments
}

// ─── Table rendering ────────────────────────────────────────────────────────

/// Parse a pipe-delimited table row into cells, trimming whitespace.
fn parse_row(line: &str) -> Vec<String> {
    let trimmed = line.trim();
    // Strip leading/trailing pipes and split on `|`
    let inner = trimmed.strip_prefix('|').unwrap_or(trimmed);
    let inner = inner.strip_suffix('|').unwrap_or(inner);
    inner.split('|').map(|c| c.trim().to_string()).collect()
}

/// Returns true when every cell in the row contains only dashes, colons, and
/// spaces (i.e. it is a separator row like `|---|:---:|---:|`).
fn is_separator_row(cells: &[String]) -> bool {
    cells
        .iter()
        .all(|c| !c.is_empty() && c.chars().all(|ch| matches!(ch, '-' | ':' | ' ')))
}

/// Render a raw markdown table string into styled `Line`s.
fn render_table(raw: &str, theme: &Theme) -> Vec<Line<'static>> {
    let rows: Vec<Vec<String>> = raw.lines().map(parse_row).collect();
    if rows.is_empty() {
        return Vec::new();
    }

    // Separate header, separator, and body rows
    let (header, body): (Option<&[String]>, Vec<&[String]>) =
        if rows.len() >= 2 && is_separator_row(&rows[1]) {
            (
                Some(&rows[0]),
                rows.iter().map(Vec::as_slice).skip(2).collect(),
            )
        } else {
            (None, rows.iter().map(Vec::as_slice).collect())
        };

    // Calculate column widths (max per column across all rows)
    let col_count = rows.iter().map(|r| r.len()).max().unwrap_or(0);
    let mut widths = vec![0usize; col_count];
    for row in &rows {
        for (i, cell) in row.iter().enumerate() {
            if i < col_count && !is_separator_row(row) {
                widths[i] = widths[i].max(cell.len());
            }
        }
    }
    // Ensure minimum column width of 3
    for w in &mut widths {
        *w = usize::max(*w, 3);
    }

    let header_style = Style::default()
        .fg(theme.primary)
        .add_modifier(Modifier::BOLD);
    let border_style = theme.text_muted();
    let cell_style = theme.text();

    let mut lines: Vec<Line<'static>> = Vec::new();

    // Build a separator line like ├───┼───┼───┤
    let build_sep = |left: &str, mid: &str, right: &str, fill: &str| -> Line<'static> {
        let mut spans = vec![Span::styled(left.to_string(), border_style)];
        for (i, w) in widths.iter().enumerate() {
            spans.push(Span::styled(fill.repeat(w + 2), border_style));
            if i < widths.len() - 1 {
                spans.push(Span::styled(mid.to_string(), border_style));
            }
        }
        spans.push(Span::styled(right.to_string(), border_style));
        Line::from(spans)
    };

    // Build a data row like │ val │ val │ val │
    let build_row = |cells: &[String], style: Style| -> Line<'static> {
        let mut spans = vec![Span::styled("│ ".to_string(), border_style)];
        for (i, w) in widths.iter().enumerate() {
            let cell = cells.get(i).map(|s| s.as_str()).unwrap_or("");
            let padded = format!("{:<width$}", cell, width = *w);
            spans.push(Span::styled(padded, style));
            if i < widths.len() - 1 {
                spans.push(Span::styled(" │ ".to_string(), border_style));
            }
        }
        spans.push(Span::styled(" │".to_string(), border_style));
        Line::from(spans)
    };

    // Top border
    lines.push(build_sep("┌", "┬", "┐", "─"));

    // Header row
    if let Some(h) = header {
        lines.push(build_row(h, header_style));
        lines.push(build_sep("├", "┼", "┤", "─"));
    }

    // Body rows
    for row in &body {
        lines.push(build_row(row, cell_style));
    }

    // Bottom border
    lines.push(build_sep("└", "┴", "┘", "─"));

    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::Theme;

    /// Flatten rendered lines into a single string for assertion.
    fn lines_to_string(lines: &[Line<'_>]) -> String {
        lines
            .iter()
            .map(|l| {
                l.spans
                    .iter()
                    .map(|s| s.content.as_ref())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn simple_table_renders() {
        let theme = Theme::new();
        let md = "| Name | Value |\n|------|-------|\n| foo  | bar   |";
        let lines = render_markdown_content(md, &theme, "");
        let text = lines_to_string(&lines);
        assert!(text.contains('┌'));
        assert!(text.contains("Name"));
        assert!(text.contains("foo"));
        assert!(text.contains("bar"));
    }

    #[test]
    fn mixed_content_with_table() {
        let theme = Theme::new();
        let md = "# Hello\n\nSome text.\n\n| A | B |\n|---|---|\n| 1 | 2 |\n\nMore text.";
        let lines = render_markdown_content(md, &theme, "");
        let text = lines_to_string(&lines);
        assert!(text.contains("Hello"));
        assert!(text.contains("Some text."));
        assert!(text.contains('┌'));
        assert!(text.contains("More text."));
    }

    #[test]
    fn no_table_passthrough() {
        let theme = Theme::new();
        let md = "Hello **world**";
        let lines = render_markdown_content(md, &theme, "");
        assert!(!lines.is_empty());
        let text = lines_to_string(&lines);
        assert!(text.contains("Hello"));
        assert!(text.contains("world"));
    }

    #[test]
    fn indent_applied() {
        let theme = Theme::new();
        let md = "| A |\n|---|\n| 1 |";
        let lines = render_markdown_content(md, &theme, "  ");
        for line in &lines {
            let first = line.spans.first().expect("line should have spans");
            assert_eq!(first.content.as_ref(), "  ");
        }
    }

    #[test]
    fn single_pipe_not_treated_as_table() {
        // A line with only one pipe should NOT be treated as a table
        let segments = split_tables("| just a pipe");
        assert!(
            matches!(segments.first(), Some(Segment::Markdown(_))),
            "single-pipe line should be markdown, not a table"
        );
    }
}
