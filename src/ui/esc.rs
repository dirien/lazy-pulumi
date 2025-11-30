//! ESC (Environments, Secrets, Configs) view rendering

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    prelude::*,
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

use crate::api::EscEnvironmentSummary;
use crate::components::StatefulList;
use crate::theme::{symbols, Theme};

/// Render the ESC environments view
pub fn render_esc_view(
    frame: &mut Frame,
    theme: &Theme,
    area: Rect,
    environments: &mut StatefulList<EscEnvironmentSummary>,
    selected_env_yaml: Option<&str>,
    selected_env_values: Option<&serde_json::Value>,
) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    render_environments_list(frame, theme, chunks[0], environments);
    render_environment_details(frame, theme, chunks[1], environments.selected(), selected_env_yaml, selected_env_values);
}

fn render_environments_list(
    frame: &mut Frame,
    theme: &Theme,
    area: Rect,
    environments: &mut StatefulList<EscEnvironmentSummary>,
) {
    // Get values before borrowing items
    let selected_idx = environments.selected_index();
    let is_empty = environments.is_empty();

    // Collect item data to owned strings
    let item_data: Vec<(String, String)> = environments
        .items()
        .iter()
        .map(|env| (env.project.clone(), env.name.clone()))
        .collect();

    let items: Vec<ListItem> = item_data
        .iter()
        .enumerate()
        .map(|(i, (project, name))| {
            let is_selected = selected_idx == Some(i);

            let content = Line::from(vec![
                Span::styled(
                    if is_selected {
                        format!("{} ", symbols::ARROW_RIGHT)
                    } else {
                        "  ".to_string()
                    },
                    theme.secondary(),
                ),
                Span::styled(project.as_str(), theme.text()),
                Span::styled("/", theme.text_muted()),
                Span::styled(name.as_str(), theme.highlight()),
            ]);

            ListItem::new(content)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(if is_empty {
                    theme.border()
                } else {
                    theme.border_focused()
                })
                .title(" ESC Environments ")
                .title_style(theme.title()),
        )
        .highlight_style(theme.selected())
        .highlight_symbol("");

    frame.render_stateful_widget(list, area, &mut environments.state);
}

fn render_environment_details(
    frame: &mut Frame,
    theme: &Theme,
    area: Rect,
    selected: Option<&EscEnvironmentSummary>,
    yaml: Option<&str>,
    values: Option<&serde_json::Value>,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(8), Constraint::Min(10)])
        .split(area);

    // Environment info
    let info_block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.border())
        .title(" Environment Details ")
        .title_style(theme.subtitle());

    let info_inner = info_block.inner(chunks[0]);
    frame.render_widget(info_block, chunks[0]);

    match selected {
        Some(env) => {
            let info_lines = vec![
                Line::from(vec![
                    Span::styled("Organization: ", theme.text_secondary()),
                    Span::styled(&env.organization, theme.text()),
                ]),
                Line::from(vec![
                    Span::styled("Project:      ", theme.text_secondary()),
                    Span::styled(&env.project, theme.primary()),
                ]),
                Line::from(vec![
                    Span::styled("Environment:  ", theme.text_secondary()),
                    Span::styled(&env.name, theme.highlight()),
                ]),
                Line::from(vec![
                    Span::styled("Created:      ", theme.text_secondary()),
                    Span::styled(
                        env.created.as_deref().unwrap_or("Unknown"),
                        theme.text(),
                    ),
                ]),
                Line::from(vec![
                    Span::styled("Modified:     ", theme.text_secondary()),
                    Span::styled(
                        env.modified.as_deref().unwrap_or("Unknown"),
                        theme.text(),
                    ),
                ]),
            ];

            let info_para = Paragraph::new(info_lines);
            frame.render_widget(info_para, info_inner);
        }
        None => {
            let empty = Paragraph::new("Select an environment to view details")
                .style(theme.text_muted())
                .alignment(Alignment::Center);
            frame.render_widget(empty, info_inner);
        }
    }

    // YAML / Values tabs
    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[1]);

    // YAML definition
    let yaml_block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.border())
        .title(" Definition (YAML) ")
        .title_style(theme.subtitle());

    let yaml_inner = yaml_block.inner(content_chunks[0]);
    frame.render_widget(yaml_block, content_chunks[0]);

    match yaml {
        Some(y) => {
            let yaml_para = Paragraph::new(y)
                .style(theme.text())
                .wrap(ratatui::widgets::Wrap { trim: false });
            frame.render_widget(yaml_para, yaml_inner);
        }
        None => {
            let hint = if selected.is_some() {
                "Press Enter to load definition"
            } else {
                "Select an environment"
            };
            let empty = Paragraph::new(hint)
                .style(theme.text_muted())
                .alignment(Alignment::Center);
            frame.render_widget(empty, yaml_inner);
        }
    }

    // Resolved values
    let values_block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.border())
        .title(" Resolved Values ")
        .title_style(theme.subtitle());

    let values_inner = values_block.inner(content_chunks[1]);
    frame.render_widget(values_block, content_chunks[1]);

    match values {
        Some(v) => {
            let formatted = serde_json::to_string_pretty(v).unwrap_or_else(|_| "Error".to_string());
            let values_para = Paragraph::new(formatted)
                .style(theme.text())
                .wrap(ratatui::widgets::Wrap { trim: false });
            frame.render_widget(values_para, values_inner);
        }
        None => {
            let hint = if selected.is_some() {
                "Press 'o' to open & resolve"
            } else {
                "Select an environment"
            };
            let empty = Paragraph::new(hint)
                .style(theme.text_muted())
                .alignment(Alignment::Center);
            frame.render_widget(empty, values_inner);
        }
    }
}
