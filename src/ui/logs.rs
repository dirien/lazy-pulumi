//! Log viewer popup rendering

use ratatui::{
    prelude::*,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
};

use crate::theme::Theme;
use crate::ui::centered_rect;

/// Render the logs popup
pub fn render_logs(
    frame: &mut Frame,
    theme: &Theme,
    log_lines: &[String],
    scroll_offset: usize,
    word_wrap: bool,
) {
    let area = centered_rect(90, 85, frame.area());

    // Clear background
    frame.render_widget(Clear, area);

    let wrap_indicator = if word_wrap { "wrap:ON" } else { "wrap:OFF" };
    let title = format!(" Logs [w:{}] (l:close, j/k:scroll, g/G:top/bottom, R:refresh) ", wrap_indicator);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.border_focused())
        .title(title)
        .title_style(theme.title());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let visible_width = inner.width as usize;
    let visible_height = inner.height as usize;

    // When word wrap is enabled, we need to calculate wrapped line count
    let (display_lines, total_display_lines) = if word_wrap {
        // Calculate total wrapped lines and create display content
        let mut wrapped_lines: Vec<(String, Style)> = Vec::new();

        for line in log_lines.iter() {
            let style = get_line_style(line, theme);

            if line.is_empty() {
                wrapped_lines.push((String::new(), style));
            } else {
                // Wrap the line manually
                let chars: Vec<char> = line.chars().collect();
                let mut start = 0;
                while start < chars.len() {
                    let end = (start + visible_width).min(chars.len());
                    let segment: String = chars[start..end].iter().collect();
                    wrapped_lines.push((segment, style));
                    start = end;
                }
            }
        }

        let total = wrapped_lines.len();

        // Clamp scroll offset
        let max_scroll = total.saturating_sub(visible_height);
        let scroll = scroll_offset.min(max_scroll);

        // Get visible wrapped lines
        let visible: Vec<Line> = wrapped_lines
            .into_iter()
            .skip(scroll)
            .take(visible_height)
            .map(|(text, style)| Line::from(Span::styled(text, style)))
            .collect();

        (visible, total)
    } else {
        // No wrapping - use original lines
        let total = log_lines.len();

        // Clamp scroll offset
        let max_scroll = total.saturating_sub(visible_height);
        let scroll = scroll_offset.min(max_scroll);

        let visible: Vec<Line> = log_lines
            .iter()
            .skip(scroll)
            .take(visible_height)
            .map(|line| {
                let style = get_line_style(line, theme);
                Line::from(Span::styled(line.as_str(), style))
            })
            .collect();

        (visible, total)
    };

    let logs_para = Paragraph::new(display_lines);
    frame.render_widget(logs_para, inner);

    // Render scrollbar if needed
    if total_display_lines > visible_height {
        let scrollbar = Scrollbar::default()
            .orientation(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓"));

        // Calculate scroll position for scrollbar
        let max_scroll = total_display_lines.saturating_sub(visible_height);
        let scroll = scroll_offset.min(max_scroll);

        let mut scrollbar_state = ScrollbarState::new(total_display_lines)
            .position(scroll)
            .viewport_content_length(visible_height);

        frame.render_stateful_widget(
            scrollbar,
            inner.inner(Margin {
                vertical: 1,
                horizontal: 0,
            }),
            &mut scrollbar_state,
        );
    }
}

/// Get the style for a log line based on its content
fn get_line_style(line: &str, theme: &Theme) -> Style {
    if line.contains("ERROR") || line.contains("error") {
        theme.error()
    } else if line.contains("WARN") || line.contains("warn") {
        theme.warning()
    } else if line.contains("INFO") || line.contains("info") {
        theme.info()
    } else if line.contains("DEBUG") || line.contains("debug") {
        theme.text_muted()
    } else {
        theme.text()
    }
}
