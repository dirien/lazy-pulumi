//! Neo AI agent view rendering

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    prelude::*,
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
};
use tui_scrollview::ScrollViewState;

use crate::api::{NeoMessage, NeoMessageType, NeoTask};
use crate::components::{StatefulList, TextInput};
use crate::theme::{symbols, Theme};

use super::centered_rect;
use super::markdown::render_markdown_content;

// Tool-related symbols
const TOOL_ICON: &str = "🔧";
const RESULT_ICON: &str = "📋";
const APPROVAL_ICON: &str = "❓";
const INFO_ICON: &str = "ℹ️";
const THINKING_ICON: &str = "🤔";

/// Render the Neo chat view
pub fn render_neo_view(
    frame: &mut Frame,
    theme: &Theme,
    area: Rect,
    tasks: &mut StatefulList<NeoTask>,
    messages: &[NeoMessage],
    input: &TextInput,
    scroll_state: &mut ScrollViewState,
    auto_scroll: &Arc<AtomicBool>,
    is_loading: bool,
    spinner_char: &str,
    hide_task_list: bool,
) {
    if hide_task_list {
        // Full-width chat when task list is hidden
        render_chat_view(frame, theme, area, messages, input, scroll_state, auto_scroll, is_loading, spinner_char);
    } else {
        // Split view with task list on left
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
            .split(area);

        render_tasks_list(frame, theme, chunks[0], tasks);
        render_chat_view(frame, theme, chunks[1], messages, input, scroll_state, auto_scroll, is_loading, spinner_char);
    }
}

fn render_tasks_list(
    frame: &mut Frame,
    theme: &Theme,
    area: Rect,
    tasks: &mut StatefulList<NeoTask>,
) {
    // Get values before borrowing items
    let selected_idx = tasks.selected_index();

    // Collect task data to owned values
    let task_data: Vec<(String, Option<String>)> = tasks
        .items()
        .iter()
        .map(|task| {
            let name = task
                .name
                .clone()
                .unwrap_or_else(|| task.id[..8.min(task.id.len())].to_string());
            (name, task.status.clone())
        })
        .collect();

    let items: Vec<ListItem> = task_data
        .iter()
        .enumerate()
        .map(|(i, (name, status))| {
            let is_selected = selected_idx == Some(i);

            let status_icon = match status.as_deref() {
                Some("completed") => symbols::CHECK,
                Some("running") | Some("in_progress") => symbols::SPINNER[0],
                Some("failed") => symbols::CROSS_MARK,
                _ => symbols::BULLET,
            };

            let status_style = match status.as_deref() {
                Some("completed") => theme.success(),
                Some("running") | Some("in_progress") => theme.warning(),
                Some("failed") => theme.error(),
                _ => theme.text_secondary(),
            };

            let content = Line::from(vec![
                Span::styled(
                    if is_selected {
                        format!("{} ", symbols::ARROW_RIGHT)
                    } else {
                        "  ".to_string()
                    },
                    theme.accent(),
                ),
                Span::styled(format!("{} ", status_icon), status_style),
                Span::styled(name.as_str(), theme.text()),
            ]);

            ListItem::new(content)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(theme.border())
                .title(" Neo Tasks ")
                .title_style(theme.title()),
        )
        .highlight_style(theme.selected())
        .highlight_symbol("");

    frame.render_stateful_widget(list, area, &mut tasks.state);
}

fn render_chat_view(
    frame: &mut Frame,
    theme: &Theme,
    area: Rect,
    messages: &[NeoMessage],
    input: &TextInput,
    scroll_state: &mut ScrollViewState,
    auto_scroll: &Arc<AtomicBool>,
    is_loading: bool,
    spinner_char: &str,
) {
    // Layout: messages area, thinking indicator (if loading), input area
    let thinking_height = if is_loading { 2 } else { 0 };
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(10),
            Constraint::Length(thinking_height),
            Constraint::Length(3),
        ])
        .split(area);

    // Messages area
    let messages_block = Block::default()
        .borders(Borders::ALL)
        .border_style(if input.is_focused() {
            theme.border()
        } else {
            theme.border_focused()
        })
        .title(" Chat ")
        .title_style(theme.subtitle());

    let messages_inner = messages_block.inner(chunks[0]);
    frame.render_widget(messages_block, chunks[0]);

    if messages.is_empty() {
        // Show welcome message or loading indicator
        if is_loading {
            // Just show empty area while loading - the thinking indicator below will show
        } else {
            let welcome_lines = vec![
                Line::from(""),
                Line::from(vec![
                    Span::styled("  Welcome to ", theme.text_secondary()),
                    Span::styled("Pulumi Neo", theme.primary()),
                    Span::styled("!", theme.text_secondary()),
                ]),
                Line::from(""),
                Line::from(Span::styled(
                    "  Neo is your AI infrastructure agent.",
                    theme.text_secondary(),
                )),
                Line::from(Span::styled(
                    "  Ask questions about your infrastructure,",
                    theme.text_secondary(),
                )),
                Line::from(Span::styled(
                    "  or request help with Pulumi operations.",
                    theme.text_secondary(),
                )),
                Line::from(""),
                Line::from(vec![
                    Span::styled("  Examples:", theme.text_muted()),
                ]),
                Line::from(vec![
                    Span::styled("    ", theme.text_muted()),
                    Span::styled(symbols::BULLET, theme.accent()),
                    Span::styled(" \"List all my AWS S3 buckets\"", theme.text()),
                ]),
                Line::from(vec![
                    Span::styled("    ", theme.text_muted()),
                    Span::styled(symbols::BULLET, theme.accent()),
                    Span::styled(" \"Check for policy violations\"", theme.text()),
                ]),
                Line::from(vec![
                    Span::styled("    ", theme.text_muted()),
                    Span::styled(symbols::BULLET, theme.accent()),
                    Span::styled(" \"Help me optimize my infrastructure\"", theme.text()),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("  Press ", theme.text_muted()),
                    Span::styled("n", theme.key_hint()),
                    Span::styled(" to start a new task, or ", theme.text_muted()),
                    Span::styled("Enter", theme.key_hint()),
                    Span::styled(" to load selected task.", theme.text_muted()),
                ]),
            ];

            let welcome = Paragraph::new(welcome_lines);
            frame.render_widget(welcome, messages_inner);
        }
    } else {
        // Build message lines - all left-aligned for simplicity
        let mut lines: Vec<Line> = Vec::new();

        for msg in messages.iter() {
            match msg.message_type {
                NeoMessageType::UserMessage => {
                    // User messages with arrow indicator
                    lines.push(Line::from(Span::styled(
                        format!("{} You:", symbols::ARROW_RIGHT),
                        theme.user_message().add_modifier(Modifier::BOLD),
                    )));
                    for line in msg.content.lines() {
                        lines.push(Line::from(Span::styled(
                            format!("    {}", line),
                            theme.text(),
                        )));
                    }
                    lines.push(Line::from(""));
                }
                NeoMessageType::AssistantMessage => {
                    // Neo messages with star indicator
                    lines.push(Line::from(Span::styled(
                        format!("{} Neo:", symbols::STAR),
                        theme.neo_message().add_modifier(Modifier::BOLD),
                    )));
                    let md_lines = render_markdown_content(&msg.content, theme, "    ");
                    lines.extend(md_lines);
                    if !msg.tool_calls.is_empty() {
                        lines.push(Line::from(""));
                        for tc in &msg.tool_calls {
                            lines.push(Line::from(vec![
                                Span::styled(format!("    {} ", TOOL_ICON), theme.warning()),
                                Span::styled("Calling: ", theme.text_muted()),
                                Span::styled(
                                    tc.name.clone(),
                                    theme.accent().add_modifier(Modifier::BOLD),
                                ),
                            ]));
                        }
                    }
                    lines.push(Line::from(""));
                }
                NeoMessageType::ToolCall => {
                    lines.push(Line::from(vec![
                        Span::styled(format!("  {} ", TOOL_ICON), theme.warning()),
                        Span::styled(msg.content.clone(), theme.text_muted()),
                    ]));
                }
                NeoMessageType::ToolResponse => {
                    lines.push(Line::from(vec![
                        Span::styled(format!("  {} ", RESULT_ICON), theme.success()),
                        Span::styled(
                            msg.tool_name.clone().unwrap_or_else(|| "Result".to_string()),
                            theme.text_secondary(),
                        ),
                        Span::styled(": ", theme.text_muted()),
                    ]));
                    let content = if msg.content.len() > 200 {
                        format!("{}...", &msg.content[..200])
                    } else {
                        msg.content.clone()
                    };
                    for line in content.lines().take(5) {
                        lines.push(Line::from(Span::styled(
                            format!("    {}", line),
                            theme.text_muted(),
                        )));
                    }
                }
                NeoMessageType::ApprovalRequest => {
                    lines.push(Line::from(vec![
                        Span::styled(format!("  {} ", APPROVAL_ICON), theme.warning()),
                        Span::styled(
                            "Approval needed: ",
                            theme.warning().add_modifier(Modifier::BOLD),
                        ),
                    ]));
                    for line in msg.content.lines() {
                        lines.push(Line::from(Span::styled(
                            format!("    {}", line),
                            theme.text(),
                        )));
                    }
                    lines.push(Line::from(""));
                }
                NeoMessageType::TaskNameChange => {
                    lines.push(Line::from(vec![
                        Span::styled(format!("  {} ", INFO_ICON), theme.text_muted()),
                        Span::styled(
                            msg.content.clone(),
                            theme.text_secondary().add_modifier(Modifier::ITALIC),
                        ),
                    ]));
                }
            }
        }

        // === Direct scrolling using Ratatui's line_count() ===
        //
        // Using the unstable-rendered-line-info feature, we get the EXACT line count
        // after wrapping, eliminating all estimation guesswork.

        let visible_height = messages_inner.height as usize;

        // Create paragraph with wrapping to get accurate line count
        let content_para = Paragraph::new(lines)
            .wrap(ratatui::widgets::Wrap { trim: false });

        // Get EXACT line count from Ratatui (accounts for actual word wrapping)
        let total_lines = content_para.line_count(messages_inner.width);
        let max_scroll = total_lines.saturating_sub(visible_height);

        // Determine scroll position
        let scroll_y: u16 = if auto_scroll.load(Ordering::Relaxed) {
            // When auto-scroll is enabled, go to exact bottom
            max_scroll as u16
        } else {
            // Manual scroll: use the stored offset, clamped to max
            let current_offset = scroll_state.offset();
            (current_offset.y as usize).min(max_scroll) as u16
        };

        // Apply scroll and render
        let content_para = content_para.scroll((scroll_y, 0));
        frame.render_widget(content_para, messages_inner);

        // Render scrollbar manually if content exceeds viewport
        if total_lines > visible_height {
            // Simple scrollbar indicator on the right edge
            let scrollbar_area = Rect::new(
                messages_inner.right().saturating_sub(1),
                messages_inner.y,
                1,
                messages_inner.height,
            );

            // For scrollbar, use estimated position (not u16::MAX)
            let scrollbar_pos = if auto_scroll.load(Ordering::Relaxed) {
                max_scroll // At bottom
            } else {
                scroll_state.offset().y as usize
            };

            // Calculate thumb position and size
            let thumb_height = ((visible_height * visible_height) / total_lines).max(1);
            let thumb_pos = if max_scroll > 0 {
                (scrollbar_pos.min(max_scroll) * (visible_height - thumb_height)) / max_scroll
            } else {
                0
            };

            // Draw scrollbar track and thumb (using Violet for on-brand look)
            for y in 0..messages_inner.height {
                let y_pos = scrollbar_area.y + y;
                let is_thumb = (y as usize) >= thumb_pos && (y as usize) < thumb_pos + thumb_height;
                let symbol = if is_thumb { "█" } else { "░" };
                let style = if is_thumb { theme.primary() } else { theme.text_muted() };

                frame.buffer_mut().set_string(
                    scrollbar_area.x,
                    y_pos,
                    symbol,
                    style,
                );
            }
        }
    }

    // Thinking indicator (always visible when loading)
    if is_loading {
        let thinking_line = Line::from(vec![
            Span::styled(format!(" {} ", THINKING_ICON), theme.primary()),
            Span::styled(format!("{} ", spinner_char), theme.warning()),
            Span::styled("Neo is thinking", theme.text()),
            Span::styled("...", theme.text_muted()),
        ]);

        let thinking_para = Paragraph::new(thinking_line)
            .style(Style::default().bg(theme.bg_medium))
            .alignment(ratatui::layout::Alignment::Center);
        frame.render_widget(thinking_para, chunks[1]);
    }

    // Input area
    let input_block = Block::default()
        .borders(Borders::ALL)
        .border_style(if input.is_focused() {
            theme.border_focused()
        } else {
            theme.border()
        })
        .title(if input.is_focused() {
            " Message (Enter to send, Esc to cancel) "
        } else {
            " Press 'i' to type, 'n' for new task "
        })
        .title_style(if input.is_focused() {
            theme.primary()
        } else {
            theme.subtitle()
        });

    let input_inner = input_block.inner(chunks[2]);
    frame.render_widget(input_block, chunks[2]);

    // Input text with cursor
    let input_value = input.value();
    let cursor_pos = input.cursor();

    if input.is_focused() {
        let before_cursor = &input_value[..cursor_pos];
        let cursor_char = input_value.chars().nth(cursor_pos).unwrap_or(' ');
        let after_cursor = if cursor_pos < input_value.len() {
            &input_value[cursor_pos + 1..]
        } else {
            ""
        };

        let input_line = Line::from(vec![
            Span::styled(before_cursor, theme.input()),
            Span::styled(cursor_char.to_string(), theme.cursor()),
            Span::styled(after_cursor, theme.input()),
        ]);

        let input_para = Paragraph::new(input_line);
        frame.render_widget(input_para, input_inner);
    } else {
        let input_para = Paragraph::new(input_value).style(theme.text_muted());
        frame.render_widget(input_para, input_inner);
    }
}

// Icons for details dialog
const STATUS_ICON: &str = "●";
const CLOCK_ICON: &str = "🕐";
const USER_ICON: &str = "👤";
const PR_ICON: &str = "🔀";
const ENTITY_ICON: &str = "◆";
const POLICY_ICON: &str = "🛡️";

/// Render the Neo task details dialog
pub fn render_neo_details_dialog(
    frame: &mut Frame,
    theme: &Theme,
    task: &NeoTask,
) {
    let area = centered_rect(25, 70, frame.area());

    // Clear background
    frame.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.border_focused())
        .title(" Task Details ")
        .title_style(theme.title());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut lines: Vec<Line> = Vec::new();

    // Status section
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        " Status",
        theme.subtitle().add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(Span::styled(
        " ─".repeat(18),
        theme.text_muted(),
    )));

    let status = task.status.as_deref().unwrap_or("Unknown");
    let status_style = match status.to_lowercase().as_str() {
        "idle" | "completed" => theme.success(),
        "running" | "in_progress" => theme.warning(),
        "failed" | "error" => theme.error(),
        _ => theme.text_secondary(),
    };

    lines.push(Line::from(vec![
        Span::styled(format!("  {} ", STATUS_ICON), status_style),
        Span::styled(status, status_style),
    ]));

    // Started on section
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        " Started on",
        theme.subtitle().add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(Span::styled(
        " ─".repeat(18),
        theme.text_muted(),
    )));

    let started_on = task.created_at.as_deref().map(|ts| {
        // Try to parse and format the timestamp nicely
        if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(ts) {
            dt.format("%b %d, %Y, %I:%M:%S %p").to_string()
        } else {
            ts.to_string()
        }
    }).unwrap_or_else(|| "Unknown".to_string());

    lines.push(Line::from(vec![
        Span::styled(format!("  {} ", CLOCK_ICON), theme.text_secondary()),
        Span::styled(started_on, theme.text()),
    ]));

    // Started by section
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        " Started by",
        theme.subtitle().add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(Span::styled(
        " ─".repeat(18),
        theme.text_muted(),
    )));

    let started_by = task.started_by.as_ref()
        .and_then(|u| u.login.clone().or(u.name.clone()))
        .unwrap_or_else(|| "Unknown".to_string());

    lines.push(Line::from(vec![
        Span::styled(format!("  {} ", USER_ICON), theme.text_secondary()),
        Span::styled(started_by, theme.text()),
    ]));

    // Linked PRs section
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        " Linked PRs",
        theme.subtitle().add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(Span::styled(
        " ─".repeat(18),
        theme.text_muted(),
    )));

    if task.linked_prs.is_empty() {
        lines.push(Line::from(vec![
            Span::styled(format!("  {} ", PR_ICON), theme.text_muted()),
            Span::styled("No linked PRs", theme.text_muted()),
        ]));
    } else {
        for pr in &task.linked_prs {
            let pr_text = format!(
                "#{} {}",
                pr.number.unwrap_or(0),
                pr.title.as_deref().unwrap_or("Untitled")
            );
            let state = pr.state.as_deref().unwrap_or("");
            let state_style = match state.to_lowercase().as_str() {
                "open" => theme.success(),
                "merged" => theme.primary(),
                "closed" => theme.error(),
                _ => theme.text_muted(),
            };
            lines.push(Line::from(vec![
                Span::styled(format!("  {} ", PR_ICON), theme.text_secondary()),
                Span::styled(pr_text, theme.text()),
                Span::styled(format!(" ({})", state), state_style),
            ]));
        }
    }

    // Involved entities section
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        " Involved entities",
        theme.subtitle().add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(Span::styled(
        " ─".repeat(18),
        theme.text_muted(),
    )));

    if task.entities.is_empty() {
        lines.push(Line::from(vec![
            Span::styled(format!("  {} ", ENTITY_ICON), theme.warning()),
            Span::styled("No linked entities", theme.warning()),
        ]));
    } else {
        for entity in &task.entities {
            let entity_type = entity.entity_type.as_deref().unwrap_or("unknown");
            let entity_name = entity.name.as_deref().unwrap_or("Unknown");
            let type_icon = match entity_type {
                "stack" => symbols::STACK,
                "environment" => symbols::GEAR,
                "repository" => "📦",
                _ => ENTITY_ICON,
            };
            lines.push(Line::from(vec![
                Span::styled(format!("  {} ", type_icon), theme.accent()),
                Span::styled(format!("{}: ", entity_type), theme.text_muted()),
                Span::styled(entity_name, theme.text()),
            ]));
        }
    }

    // Active policies section
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        " Active policies",
        theme.subtitle().add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(Span::styled(
        " ─".repeat(18),
        theme.text_muted(),
    )));

    if task.policies.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("  ", theme.text_muted()),
            Span::styled("Set up policy groups to enforce", theme.text_muted()),
        ]));
        lines.push(Line::from(vec![
            Span::styled("  ", theme.text_muted()),
            Span::styled("guardrails on infrastructure changes.", theme.text_muted()),
        ]));
    } else {
        for policy in &task.policies {
            let policy_name = policy.name.as_deref().unwrap_or("Unknown");
            let enforcement = policy.enforcement_level.as_deref().unwrap_or("");
            let enforcement_style = match enforcement.to_lowercase().as_str() {
                "mandatory" => theme.error(),
                "advisory" => theme.warning(),
                _ => theme.text_muted(),
            };
            lines.push(Line::from(vec![
                Span::styled(format!("  {} ", POLICY_ICON), theme.text_secondary()),
                Span::styled(policy_name, theme.text()),
                if !enforcement.is_empty() {
                    Span::styled(format!(" ({})", enforcement), enforcement_style)
                } else {
                    Span::raw("")
                },
            ]));
        }
    }

    // Footer hint
    lines.push(Line::from(""));
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        " Press d or Esc to close ",
        theme.text_muted(),
    )));

    let details_para = Paragraph::new(lines).wrap(ratatui::widgets::Wrap { trim: false });
    frame.render_widget(details_para, inner);
}
