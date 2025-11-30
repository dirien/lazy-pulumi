//! Dashboard view rendering

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    prelude::*,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::app::AppState;
use crate::theme::{symbols, Theme};

/// Render the dashboard view
pub fn render_dashboard(frame: &mut Frame, theme: &Theme, area: Rect, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8),  // Stats cards
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
    icon: &str,
    accent_color: Color,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.border())
        .title(format!(" {} ", title))
        .title_style(theme.subtitle());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let content = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Length(2)])
        .margin(1)
        .split(inner);

    // Icon and value
    let value_line = Line::from(vec![
        Span::styled(format!("{} ", icon), Style::default().fg(accent_color)),
        Span::styled(
            value,
            Style::default()
                .fg(theme.text_primary)
                .add_modifier(Modifier::BOLD),
        ),
    ]);

    let value_para = Paragraph::new(value_line).alignment(Alignment::Center);
    frame.render_widget(value_para, content[0]);
}

fn render_recent_activity(frame: &mut Frame, theme: &Theme, area: Rect, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(area);

    // Recent stacks
    let stacks_block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.border())
        .title(" Recent Stacks ")
        .title_style(theme.subtitle());

    let stacks_inner = stacks_block.inner(chunks[0]);
    frame.render_widget(stacks_block, chunks[0]);

    let stack_lines: Vec<Line> = state
        .stacks
        .iter()
        .take(10)
        .map(|s| {
            Line::from(vec![
                Span::styled(
                    format!("{} ", symbols::ARROW_RIGHT),
                    theme.primary(),
                ),
                Span::styled(&s.project_name, theme.text()),
                Span::styled("/", theme.text_muted()),
                Span::styled(&s.stack_name, theme.highlight()),
                Span::styled(
                    format!("  {}", s.last_update_formatted()),
                    theme.text_muted(),
                ),
            ])
        })
        .collect();

    let stacks_para = Paragraph::new(stack_lines).wrap(ratatui::widgets::Wrap { trim: true });
    frame.render_widget(stacks_para, stacks_inner);

    // Activity sparkline / info panel
    let info_block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.border())
        .title(" Quick Info ")
        .title_style(theme.subtitle());

    let info_inner = info_block.inner(chunks[1]);
    frame.render_widget(info_block, chunks[1]);

    let info_lines = vec![
        Line::from(vec![
            Span::styled("Press ", theme.text_secondary()),
            Span::styled("Tab", theme.key_hint()),
            Span::styled(" to switch views", theme.text_secondary()),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Press ", theme.text_secondary()),
            Span::styled("?", theme.key_hint()),
            Span::styled(" for help", theme.text_secondary()),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Press ", theme.text_secondary()),
            Span::styled("q", theme.key_hint()),
            Span::styled(" to quit", theme.text_secondary()),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Press ", theme.text_secondary()),
            Span::styled("r", theme.key_hint()),
            Span::styled(" to refresh", theme.text_secondary()),
        ]),
    ];

    let info_para = Paragraph::new(info_lines);
    frame.render_widget(info_para, info_inner);
}
