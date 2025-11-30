//! Stacks view rendering

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    prelude::*,
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Row, Table},
};

use crate::api::Stack;
use crate::components::StatefulList;
use crate::theme::{symbols, Theme};

/// Render the stacks view
pub fn render_stacks_view(
    frame: &mut Frame,
    theme: &Theme,
    area: Rect,
    stacks: &mut StatefulList<Stack>,
    selected_stack_updates: &[(i32, String, String)], // (version, result, time)
) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    render_stacks_list(frame, theme, chunks[0], stacks);
    render_stack_details(frame, theme, chunks[1], stacks.selected(), selected_stack_updates);
}

fn render_stacks_list(
    frame: &mut Frame,
    theme: &Theme,
    area: Rect,
    stacks: &mut StatefulList<Stack>,
) {
    // Get selected index and is_empty before borrowing items
    let selected_idx = stacks.selected_index();
    let is_empty = stacks.is_empty();

    // Collect item data to owned strings to avoid borrow issues
    let item_data: Vec<(String, String)> = stacks
        .items()
        .iter()
        .map(|stack| (stack.project_name.clone(), stack.stack_name.clone()))
        .collect();

    let items: Vec<ListItem> = item_data
        .iter()
        .enumerate()
        .map(|(i, (project, stack_name))| {
            let is_selected = selected_idx == Some(i);

            let content = Line::from(vec![
                Span::styled(
                    if is_selected {
                        format!("{} ", symbols::ARROW_RIGHT)
                    } else {
                        "  ".to_string()
                    },
                    theme.primary(),
                ),
                Span::styled(project.as_str(), theme.text()),
                Span::styled("/", theme.text_muted()),
                Span::styled(stack_name.as_str(), theme.highlight()),
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
                .title(" Stacks ")
                .title_style(theme.title()),
        )
        .highlight_style(theme.selected())
        .highlight_symbol("");

    frame.render_stateful_widget(list, area, &mut stacks.state);
}

fn render_stack_details(
    frame: &mut Frame,
    theme: &Theme,
    area: Rect,
    selected: Option<&Stack>,
    updates: &[(i32, String, String)],
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(10), Constraint::Min(5)])
        .split(area);

    // Stack info
    let info_block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.border())
        .title(" Stack Details ")
        .title_style(theme.subtitle());

    let info_inner = info_block.inner(chunks[0]);
    frame.render_widget(info_block, chunks[0]);

    match selected {
        Some(stack) => {
            let info_lines = vec![
                Line::from(vec![
                    Span::styled("Organization: ", theme.text_secondary()),
                    Span::styled(&stack.org_name, theme.text()),
                ]),
                Line::from(vec![
                    Span::styled("Project:      ", theme.text_secondary()),
                    Span::styled(&stack.project_name, theme.primary()),
                ]),
                Line::from(vec![
                    Span::styled("Stack:        ", theme.text_secondary()),
                    Span::styled(&stack.stack_name, theme.highlight()),
                ]),
                Line::from(vec![
                    Span::styled("Last Update:  ", theme.text_secondary()),
                    Span::styled(stack.last_update_formatted(), theme.text()),
                ]),
                Line::from(vec![
                    Span::styled("Resources:    ", theme.text_secondary()),
                    Span::styled(
                        stack
                            .resource_count
                            .map(|c| c.to_string())
                            .unwrap_or_else(|| "N/A".to_string()),
                        theme.info(),
                    ),
                ]),
            ];

            let info_para = Paragraph::new(info_lines);
            frame.render_widget(info_para, info_inner);
        }
        None => {
            let empty = Paragraph::new("Select a stack to view details")
                .style(theme.text_muted())
                .alignment(Alignment::Center);
            frame.render_widget(empty, info_inner);
        }
    }

    // Updates history
    let updates_block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.border())
        .title(" Update History ")
        .title_style(theme.subtitle());

    let updates_inner = updates_block.inner(chunks[1]);
    frame.render_widget(updates_block, chunks[1]);

    if updates.is_empty() {
        let empty = Paragraph::new("No updates yet")
            .style(theme.text_muted())
            .alignment(Alignment::Center);
        frame.render_widget(empty, updates_inner);
    } else {
        let rows: Vec<Row> = updates
            .iter()
            .map(|(version, result, time)| {
                let result_style = match result.to_lowercase().as_str() {
                    "succeeded" => theme.success(),
                    "failed" => theme.error(),
                    _ => theme.warning(),
                };

                Row::new(vec![
                    format!("v{}", version),
                    result.clone(),
                    time.clone(),
                ])
                .style(result_style)
            })
            .collect();

        let table = Table::new(
            rows,
            [
                Constraint::Length(6),
                Constraint::Length(12),
                Constraint::Min(10),
            ],
        )
        .header(
            Row::new(vec!["Ver", "Result", "Time"])
                .style(theme.subtitle())
                .bottom_margin(1),
        );

        frame.render_widget(table, updates_inner);
    }
}
