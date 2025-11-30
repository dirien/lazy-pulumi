//! NEO AI agent view rendering

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect, Size},
    prelude::*,
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};
use tui_scrollview::{ScrollView, ScrollViewState};
use unicode_width::UnicodeWidthStr;

use crate::api::{NeoMessage, NeoMessageType, NeoTask};
use crate::components::{StatefulList, TextInput};
use crate::theme::{symbols, Theme};

use super::markdown::render_markdown_content;

// Tool-related symbols
const TOOL_ICON: &str = "🔧";
const RESULT_ICON: &str = "📋";
const APPROVAL_ICON: &str = "❓";
const INFO_ICON: &str = "ℹ️";
const THINKING_ICON: &str = "🤔";

/// Render the NEO chat view
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
) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(area);

    render_tasks_list(frame, theme, chunks[0], tasks);
    render_chat_view(frame, theme, chunks[1], messages, input, scroll_state, auto_scroll, is_loading, spinner_char);
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
                .title(" NEO Tasks ")
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

    if messages.is_empty() && !is_loading {
        let welcome_lines = vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("  Welcome to ", theme.text_secondary()),
                Span::styled("Pulumi NEO", theme.primary()),
                Span::styled("!", theme.text_secondary()),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "  NEO is your AI infrastructure agent.",
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
                Span::styled(" to send a message.", theme.text_muted()),
            ]),
        ];

        let welcome = Paragraph::new(welcome_lines);
        frame.render_widget(welcome, messages_inner);
    } else {
        // Build message lines with markdown support
        let mut lines: Vec<Line> = Vec::new();

        for msg in messages.iter() {
            match msg.message_type {
                NeoMessageType::UserMessage => {
                    lines.push(Line::from(Span::styled(
                        format!("{} You: ", symbols::ARROW_RIGHT),
                        theme.user_message().add_modifier(Modifier::BOLD),
                    )));
                    for line in msg.content.lines() {
                        lines.push(Line::from(Span::styled(format!("    {}", line), theme.text())));
                    }
                    lines.push(Line::from(""));
                }
                NeoMessageType::AssistantMessage => {
                    lines.push(Line::from(Span::styled(
                        format!("{} NEO: ", symbols::STAR),
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
                                Span::styled(tc.name.clone(), theme.accent().add_modifier(Modifier::BOLD)),
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
                        Span::styled("Approval needed: ", theme.warning().add_modifier(Modifier::BOLD)),
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
                        Span::styled(msg.content.clone(), theme.text_secondary().add_modifier(Modifier::ITALIC)),
                    ]));
                }
            }
        }

        // Calculate wrapped content height using proper unicode width
        let available_width = messages_inner.width.saturating_sub(2) as usize; // -2 for scrollbar + margin
        let mut wrapped_height: u16 = 0;
        for line in &lines {
            // Calculate the display width of this line using unicode width
            let line_width: usize = line.spans.iter().map(|s| s.content.width()).sum();
            if line_width == 0 {
                wrapped_height += 1;
            } else if available_width == 0 {
                wrapped_height += 1;
            } else {
                // How many visual lines will this take when wrapped?
                // Add 1 to ensure we don't undercount
                wrapped_height += ((line_width + available_width - 1) / available_width) as u16;
            }
        }
        // Add extra buffer for any edge cases in wrapping
        wrapped_height = (wrapped_height + (wrapped_height / 10).max(5)).max(1);

        // Create ScrollView - width matches viewport (no horizontal scroll), height is wrapped content
        let content_width = messages_inner.width;
        let scroll_height = wrapped_height.max(messages_inner.height);
        let mut scroll_view = ScrollView::new(Size::new(content_width, scroll_height));

        // Render content into scroll view with word wrap
        // Use the full scroll height to ensure all content is visible
        let content_area = Rect::new(0, 0, content_width, scroll_height);
        let content_para = Paragraph::new(lines)
            .wrap(ratatui::widgets::Wrap { trim: false });
        scroll_view.render_widget(content_para, content_area);

        // Auto-scroll to bottom if enabled
        if auto_scroll.load(Ordering::Relaxed) {
            scroll_state.scroll_to_bottom();
        }

        // Render the scroll view (it has its own scrollbar)
        frame.render_stateful_widget(scroll_view, messages_inner, scroll_state);
    }

    // Thinking indicator (always visible when loading)
    if is_loading {
        let thinking_line = Line::from(vec![
            Span::styled(format!(" {} ", THINKING_ICON), theme.primary()),
            Span::styled(format!("{} ", spinner_char), theme.warning()),
            Span::styled("NEO is thinking", theme.text()),
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
