//! Dashboard view rendering

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    prelude::*,
    symbols::Marker,
    text::{Line, Span},
    widgets::{Axis, Block, Borders, Chart, Dataset, GraphType, Paragraph},
};
use tui_big_text::{BigText, PixelSize};

use crate::app::AppState;
use crate::theme::{symbols, Theme};

/// Format a unix timestamp as relative time (e.g., "2 days ago", "3 hours ago")
fn format_time_ago(timestamp: i64) -> String {
    let now = chrono::Utc::now().timestamp();
    let diff = now - timestamp;

    if diff < 0 {
        return "just now".to_string();
    }

    let minutes = diff / 60;
    let hours = diff / 3600;
    let days = diff / 86400;

    if days > 0 {
        if days == 1 {
            "1 day ago".to_string()
        } else {
            format!("{} days ago", days)
        }
    } else if hours > 0 {
        if hours == 1 {
            "1 hour ago".to_string()
        } else {
            format!("{} hours ago", hours)
        }
    } else if minutes > 0 {
        if minutes == 1 {
            "1 min ago".to_string()
        } else {
            format!("{} mins ago", minutes)
        }
    } else {
        "just now".to_string()
    }
}

/// Render the dashboard view
pub fn render_dashboard(frame: &mut Frame, theme: &Theme, area: Rect, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(10), // Stats cards with big text
            Constraint::Min(10),    // Recent activity
        ])
        .split(area);

    render_stats_cards(frame, theme, chunks[0], state);
    render_recent_activity(frame, theme, chunks[1], state);
}

fn render_stats_cards(frame: &mut Frame, theme: &Theme, area: Rect, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ])
        .split(area);

    // Stacks card
    let stacks_count = state.stacks.len();
    render_stat_card(
        frame,
        theme,
        chunks[0],
        "Stacks",
        &stacks_count.to_string(),
        symbols::DIAMOND,
        theme.primary,
    );

    // ESC Environments card
    let env_count = state.esc_environments.len();
    render_stat_card(
        frame,
        theme,
        chunks[1],
        "Environments",
        &env_count.to_string(),
        symbols::STAR,
        theme.secondary,
    );

    // Neo Tasks card
    let neo_count = state.neo_tasks.len();
    render_stat_card(
        frame,
        theme,
        chunks[2],
        "Neo Tasks",
        &neo_count.to_string(),
        symbols::BULLET,
        theme.accent,
    );

    // Resources card
    let resource_count = state.resources.len();
    render_stat_card(
        frame,
        theme,
        chunks[3],
        "Resources",
        &resource_count.to_string(),
        symbols::CHECK,
        theme.success,
    );
}

fn render_stat_card(
    frame: &mut Frame,
    theme: &Theme,
    area: Rect,
    title: &str,
    value: &str,
    _icon: &str,
    accent_color: Color,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.border())
        .title(format!(" {} ", title))
        .title_style(theme.subtitle());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // BigText with Quadrant pixel size is 4 rows tall
    let big_text_height = 4_u16;

    // Center vertically within the inner area
    let vertical_padding = inner.height.saturating_sub(big_text_height) / 2;
    let centered_area = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(vertical_padding),
            Constraint::Length(big_text_height),
            Constraint::Min(0),
        ])
        .split(inner)[1];

    // Use BigText for the value
    let big_text = BigText::builder()
        .pixel_size(PixelSize::Quadrant)
        .style(Style::default().fg(accent_color))
        .lines(vec![Line::from(value)])
        .centered()
        .build();

    frame.render_widget(big_text, centered_area);
}

fn render_recent_activity(frame: &mut Frame, theme: &Theme, area: Rect, state: &AppState) {
    // Layout: Resource chart (full width) on top, then updates + quick info below
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(10), Constraint::Min(5)])
        .split(area);

    // Resource count over time chart (full width)
    render_resource_chart(frame, theme, main_chunks[0], state);

    // Bottom row: Recent updates (left) + Quick info (right, smaller)
    let bottom_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(75), Constraint::Percentage(25)])
        .split(main_chunks[1]);

    // Recent stack updates
    let updates_block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.border())
        .title(" Recent Stack Updates ")
        .title_style(theme.subtitle());

    let updates_inner = updates_block.inner(bottom_chunks[0]);
    frame.render_widget(updates_block, bottom_chunks[0]);

    // Deduplicate: only show the latest update per project/stack
    let mut seen_stacks: std::collections::HashSet<String> = std::collections::HashSet::new();
    let unique_updates: Vec<_> = state
        .recent_updates
        .iter()
        .filter(|u| {
            let key = format!("{}/{}", u.project_name, u.stack_name);
            if seen_stacks.contains(&key) {
                false
            } else {
                seen_stacks.insert(key);
                true
            }
        })
        .take(5)
        .collect();

    // Build two lines per update (like Pulumi Cloud UI)
    let mut update_lines: Vec<Line> = Vec::new();
    for u in unique_updates.iter() {
        // Format relative time
        let time_ago = format_time_ago(u.start_time);
        let username = u.requested_by.as_deref().unwrap_or("unknown");

        // Line 1: project / stack / Update #N
        update_lines.push(Line::from(vec![
            Span::styled(format!("{} ", symbols::DIAMOND), theme.primary()),
            Span::styled(&u.project_name, theme.text()),
            Span::styled(" / ", theme.text_muted()),
            Span::styled(&u.stack_name, theme.highlight()),
            Span::styled(" / ", theme.text_muted()),
            Span::styled(format!("Update #{}", u.version), theme.text_secondary()),
        ]));

        // Line 2: username updated X days ago
        update_lines.push(Line::from(vec![
            Span::styled("  ", Style::default()), // indent
            Span::styled(username, theme.text_muted()),
            Span::styled(" updated ", theme.text_muted()),
            Span::styled(time_ago, theme.text_muted()),
        ]));
    }

    if update_lines.is_empty() {
        let empty_msg = Paragraph::new(Line::from(vec![
            Span::styled("No recent updates", theme.text_muted()),
        ]));
        frame.render_widget(empty_msg, updates_inner);
    } else {
        let updates_para = Paragraph::new(update_lines);
        frame.render_widget(updates_para, updates_inner);
    }

    // Quick Info panel (smaller, on right)
    let info_block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.border())
        .title(" Quick Info ")
        .title_style(theme.subtitle());

    let info_inner = info_block.inner(bottom_chunks[1]);
    frame.render_widget(info_block, bottom_chunks[1]);

    let info_lines = vec![
        Line::from(vec![
            Span::styled("Tab", theme.key_hint()),
            Span::styled(" views", theme.text_muted()),
        ]),
        Line::from(vec![
            Span::styled("?", theme.key_hint()),
            Span::styled(" help", theme.text_muted()),
        ]),
        Line::from(vec![
            Span::styled("r", theme.key_hint()),
            Span::styled(" refresh", theme.text_muted()),
        ]),
    ];

    let info_para = Paragraph::new(info_lines);
    frame.render_widget(info_para, info_inner);
}

/// Render resource count over time chart using Chart widget
fn render_resource_chart(frame: &mut Frame, theme: &Theme, area: Rect, state: &AppState) {
    if state.resource_summary.is_empty() {
        let empty_block = Block::default()
            .borders(Borders::ALL)
            .border_style(theme.border())
            .title(" Resource Count Over Time ")
            .title_style(theme.subtitle());
        let inner = empty_block.inner(area);
        frame.render_widget(empty_block, area);
        let empty_msg = Paragraph::new(Line::from(vec![Span::styled(
            "No resource data",
            theme.text_muted(),
        )]));
        frame.render_widget(empty_msg, inner);
        return;
    }

    // Convert data to (f64, f64) tuples for Chart widget
    let data: Vec<(f64, f64)> = state
        .resource_summary
        .iter()
        .enumerate()
        .map(|(i, point)| (i as f64, point.resources as f64))
        .collect();

    // Calculate bounds
    let max_x = data.len() as f64;
    let max_y = data
        .iter()
        .map(|(_, y)| *y)
        .fold(0.0_f64, |a, b| a.max(b));
    let min_y = data
        .iter()
        .map(|(_, y)| *y)
        .fold(f64::MAX, |a, b| a.min(b));

    // Add some padding to y bounds
    let y_padding = ((max_y - min_y) * 0.1).max(5.0);
    let y_min = (min_y - y_padding).max(0.0);
    let y_max = max_y + y_padding;

    // Get date labels for x-axis
    let first_label = state
        .resource_summary
        .first()
        .map(|p| p.date_label())
        .unwrap_or_default();
    let last_label = state
        .resource_summary
        .last()
        .map(|p| p.date_label())
        .unwrap_or_default();

    // Current resource count for title
    let current_count = state
        .resource_summary
        .last()
        .map(|p| p.resources)
        .unwrap_or(0);

    let datasets = vec![Dataset::default()
        .name(format!("{} resources", current_count))
        .marker(Marker::Braille)
        .graph_type(GraphType::Line)
        .style(Style::default().fg(theme.primary))
        .data(&data)];

    let chart = Chart::new(datasets)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(theme.border())
                .title(" Resource Count Over Time ")
                .title_style(theme.subtitle()),
        )
        .x_axis(
            Axis::default()
                .style(theme.text_muted())
                .bounds([0.0, max_x])
                .labels(vec![
                    Span::styled(first_label, theme.text_muted()),
                    Span::styled(last_label, theme.text_muted()),
                ]),
        )
        .y_axis(
            Axis::default()
                .style(theme.text_muted())
                .bounds([y_min, y_max])
                .labels(vec![
                    Span::styled(format!("{:.0}", y_min), theme.text_muted()),
                    Span::styled(format!("{:.0}", y_max), theme.text_muted()),
                ]),
        );

    frame.render_widget(chart, area);
}
