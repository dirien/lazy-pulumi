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
use crate::app::SlashCommandsDialogView;
use crate::components::{StatefulList, TextEditor, TextInput};
use crate::theme::{symbols, Theme};

use super::centered_rect;
use super::markdown::render_markdown_content;

// Tool-related symbols
const TOOL_ICON: &str = "🔧";
const RESULT_ICON: &str = "📋";
const ERROR_ICON: &str = "❌";
const APPROVAL_ICON: &str = "❓";
const INFO_ICON: &str = "ℹ️";
const THINKING_ICON: &str = "🤔";

/// Slash command for the picker
use crate::api::NeoSlashCommand;

/// Props for command picker state
pub struct CommandPickerProps<'a> {
    pub show: bool,
    pub filtered_commands: &'a [NeoSlashCommand],
    pub index: usize,
    pub all_commands: &'a [NeoSlashCommand],
    pub pending_commands: &'a [NeoSlashCommand],
}

/// Props for rendering the Neo view
pub struct NeoViewProps<'a> {
    pub tasks: &'a mut StatefulList<NeoTask>,
    pub messages: &'a [NeoMessage],
    pub input: &'a TextInput,
    pub scroll_state: &'a mut ScrollViewState,
    pub auto_scroll: &'a Arc<AtomicBool>,
    pub is_loading: bool,
    pub spinner_char: &'a str,
    pub hide_task_list: bool,
    pub command_picker: CommandPickerProps<'a>,
}

/// Props for chat view (internal)
struct ChatViewProps<'a> {
    messages: &'a [NeoMessage],
    input: &'a TextInput,
    scroll_state: &'a mut ScrollViewState,
    auto_scroll: &'a Arc<AtomicBool>,
    is_loading: bool,
    spinner_char: &'a str,
    command_picker: CommandPickerProps<'a>,
}

/// Render the Neo chat view
pub fn render_neo_view(frame: &mut Frame, theme: &Theme, area: Rect, props: NeoViewProps<'_>) {
    let chat_props = ChatViewProps {
        messages: props.messages,
        input: props.input,
        scroll_state: props.scroll_state,
        auto_scroll: props.auto_scroll,
        is_loading: props.is_loading,
        spinner_char: props.spinner_char,
        command_picker: props.command_picker,
    };

    if props.hide_task_list {
        // Full-width chat when task list is hidden
        render_chat_view(frame, theme, area, chat_props);
    } else {
        // Split view with task list on left
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
            .split(area);

        render_tasks_list(frame, theme, chunks[0], props.tasks);
        render_chat_view(frame, theme, chunks[1], chat_props);
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

// Command picker icon
const COMMAND_ICON: &str = "⌘";

fn render_chat_view(frame: &mut Frame, theme: &Theme, area: Rect, props: ChatViewProps<'_>) {
    // Layout: messages area, thinking indicator (if loading), command picker (if showing), input area
    let thinking_height = if props.is_loading { 2 } else { 0 };
    let command_picker_height = if props.command_picker.show {
        // Show up to 8 commands + 2 for borders
        (props.command_picker.filtered_commands.len().min(8) + 2) as u16
    } else {
        0
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(10),
            Constraint::Length(thinking_height),
            Constraint::Length(command_picker_height),
            Constraint::Length(3),
        ])
        .split(area);

    // Messages area
    let messages_block = Block::default()
        .borders(Borders::ALL)
        .border_style(if props.input.is_focused() {
            theme.border()
        } else {
            theme.border_focused()
        })
        .title(" Chat ")
        .title_style(theme.subtitle());

    let messages_inner = messages_block.inner(chunks[0]);
    frame.render_widget(messages_block, chunks[0]);

    if props.messages.is_empty() {
        // Show welcome message or loading indicator
        if props.is_loading {
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
                Line::from(vec![Span::styled("  Examples:", theme.text_muted())]),
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

        for msg in props.messages.iter() {
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
                            msg.tool_name
                                .clone()
                                .unwrap_or_else(|| "Result".to_string()),
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
                NeoMessageType::ToolError => {
                    // Show tool error with red error styling
                    lines.push(Line::from(vec![
                        Span::styled(format!("  {} ", ERROR_ICON), theme.error()),
                        Span::styled(
                            format!(
                                "Error running {}",
                                msg.tool_name.clone().unwrap_or_else(|| "tool".to_string())
                            ),
                            theme.error().add_modifier(Modifier::BOLD),
                        ),
                    ]));
                    // Show the error message (don't truncate as much for errors)
                    for line in msg.content.lines().take(10) {
                        lines.push(Line::from(Span::styled(
                            format!("    {}", line),
                            theme.error(),
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
        let content_para = Paragraph::new(lines).wrap(ratatui::widgets::Wrap { trim: false });

        // Get EXACT line count from Ratatui (accounts for actual word wrapping)
        let total_lines = content_para.line_count(messages_inner.width);
        let max_scroll = total_lines.saturating_sub(visible_height);

        // Determine scroll position
        let scroll_y: u16 = if props.auto_scroll.load(Ordering::Relaxed) {
            // When auto-scroll is enabled, go to exact bottom
            max_scroll as u16
        } else {
            // Manual scroll: use the stored offset, clamped to max
            let current_offset = props.scroll_state.offset();
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
            let scrollbar_pos = if props.auto_scroll.load(Ordering::Relaxed) {
                max_scroll // At bottom
            } else {
                props.scroll_state.offset().y as usize
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
                let style = if is_thumb {
                    theme.primary()
                } else {
                    theme.text_muted()
                };

                frame
                    .buffer_mut()
                    .set_string(scrollbar_area.x, y_pos, symbol, style);
            }
        }
    }

    // Thinking indicator (always visible when loading)
    if props.is_loading {
        let thinking_line = Line::from(vec![
            Span::styled(format!(" {} ", THINKING_ICON), theme.primary()),
            Span::styled(format!("{} ", props.spinner_char), theme.warning()),
            Span::styled("Neo is thinking", theme.text()),
            Span::styled("...", theme.text_muted()),
        ]);

        let thinking_para = Paragraph::new(thinking_line)
            .style(Style::default().bg(theme.bg_medium))
            .alignment(ratatui::layout::Alignment::Center);
        frame.render_widget(thinking_para, chunks[1]);
    }

    // Slash command picker (shown above input when typing '/')
    if props.command_picker.show && !props.command_picker.filtered_commands.is_empty() {
        render_command_picker(
            frame,
            theme,
            chunks[2],
            props.command_picker.filtered_commands,
            props.command_picker.index,
        );
    }

    // Determine the input title based on context
    let input_title = if props.input.is_focused() {
        if props.command_picker.show {
            " ↑↓: select | Tab: complete | Enter: run "
        } else if !props.command_picker.all_commands.is_empty() {
            " Type / for commands | Enter to send "
        } else {
            " Message (Enter to send, Esc to cancel) "
        }
    } else if !props.command_picker.all_commands.is_empty() {
        " Press 'i' to type, '/' for commands "
    } else {
        " Press 'i' to type, 'n' for new task "
    };

    // Input area
    let input_block = Block::default()
        .borders(Borders::ALL)
        .border_style(if props.input.is_focused() {
            theme.border_focused()
        } else {
            theme.border()
        })
        .title(input_title)
        .title_style(if props.input.is_focused() {
            theme.primary()
        } else {
            theme.subtitle()
        });

    let input_inner = input_block.inner(chunks[3]);
    frame.render_widget(input_block, chunks[3]);

    // Input text with cursor - highlight slash commands with purple background
    let input_value = props.input.value();
    let cursor_pos = props.input.cursor();

    // Build a list of command names to highlight
    let command_names: Vec<&str> = props
        .command_picker
        .pending_commands
        .iter()
        .map(|c| c.name.as_str())
        .collect();

    if props.input.is_focused() {
        // Render input with slash command highlighting
        let spans = render_input_with_commands(input_value, cursor_pos, &command_names, theme);
        let input_line = Line::from(spans);
        let input_para = Paragraph::new(input_line);
        frame.render_widget(input_para, input_inner);
    } else {
        // When not focused, still show command highlighting
        let spans = render_input_with_commands_unfocused(input_value, &command_names, theme);
        let input_line = Line::from(spans);
        let input_para = Paragraph::new(input_line);
        frame.render_widget(input_para, input_inner);
    }
}

/// Render input text with slash commands highlighted in purple (focused mode with cursor)
fn render_input_with_commands<'a>(
    input: &'a str,
    cursor_pos: usize,
    command_names: &[&str],
    theme: &Theme,
) -> Vec<Span<'a>> {
    use crate::theme::brand;

    // Purple style for commands
    let command_style = Style::default().fg(Color::White).bg(brand::VIOLET);

    let mut spans = Vec::new();
    let mut i = 0;
    let chars: Vec<char> = input.chars().collect();

    while i < chars.len() {
        if chars[i] == '/' {
            // Check if this is a known command
            let remaining: String = chars[i..].iter().collect();
            let mut found_command = false;

            for &cmd_name in command_names {
                let pattern = format!("/{}", cmd_name);
                if remaining.starts_with(&pattern) {
                    // Check that command ends with space or end of string
                    let after_cmd = remaining.get(pattern.len()..).unwrap_or("");
                    if after_cmd.is_empty() || after_cmd.starts_with(' ') {
                        // Found a matching command - render it with purple background
                        let cmd_start = i;
                        let cmd_end = i + pattern.len();

                        // Render the command with cursor handling
                        for (j, c) in pattern.chars().enumerate() {
                            let char_pos = cmd_start + j;
                            if char_pos == cursor_pos {
                                spans.push(Span::styled(c.to_string(), theme.cursor()));
                            } else {
                                spans.push(Span::styled(c.to_string(), command_style));
                            }
                        }

                        i = cmd_end;
                        found_command = true;
                        break;
                    }
                }
            }

            if !found_command {
                // Regular '/' character
                if i == cursor_pos {
                    spans.push(Span::styled("/", theme.cursor()));
                } else {
                    spans.push(Span::styled("/", theme.input()));
                }
                i += 1;
            }
        } else {
            // Regular character
            if i == cursor_pos {
                spans.push(Span::styled(chars[i].to_string(), theme.cursor()));
            } else {
                spans.push(Span::styled(chars[i].to_string(), theme.input()));
            }
            i += 1;
        }
    }

    // Add cursor at end if cursor is at end of input
    if cursor_pos >= chars.len() {
        spans.push(Span::styled(" ", theme.cursor()));
    }

    spans
}

/// Render input text with slash commands highlighted (unfocused mode, no cursor)
fn render_input_with_commands_unfocused<'a>(
    input: &'a str,
    command_names: &[&str],
    theme: &Theme,
) -> Vec<Span<'a>> {
    use crate::theme::brand;

    // Purple style for commands
    let command_style = Style::default().fg(Color::White).bg(brand::VIOLET);

    let mut spans = Vec::new();
    let mut i = 0;
    let chars: Vec<char> = input.chars().collect();

    while i < chars.len() {
        if chars[i] == '/' {
            // Check if this is a known command
            let remaining: String = chars[i..].iter().collect();
            let mut found_command = false;

            for &cmd_name in command_names {
                let pattern = format!("/{}", cmd_name);
                if remaining.starts_with(&pattern) {
                    // Check that command ends with space or end of string
                    let after_cmd = remaining.get(pattern.len()..).unwrap_or("");
                    if after_cmd.is_empty() || after_cmd.starts_with(' ') {
                        // Found a matching command
                        spans.push(Span::styled(pattern.clone(), command_style));
                        i += pattern.len();
                        found_command = true;
                        break;
                    }
                }
            }

            if !found_command {
                spans.push(Span::styled("/", theme.text_muted()));
                i += 1;
            }
        } else {
            spans.push(Span::styled(chars[i].to_string(), theme.text_muted()));
            i += 1;
        }
    }

    spans
}

/// Render the slash command picker popup
fn render_command_picker(
    frame: &mut Frame,
    theme: &Theme,
    area: Rect,
    commands: &[NeoSlashCommand],
    selected_index: usize,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.border_focused())
        .title(" Slash Commands ")
        .title_style(theme.title());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Build command list items
    let items: Vec<ListItem> = commands
        .iter()
        .enumerate()
        .take(8) // Show max 8 commands
        .map(|(i, cmd)| {
            let is_selected = i == selected_index;

            let prefix = if is_selected {
                format!("{} ", symbols::ARROW_RIGHT)
            } else {
                "  ".to_string()
            };

            // Truncate description if too long
            let max_desc_len = 50;
            let desc = if cmd.description.len() > max_desc_len {
                format!("{}...", &cmd.description[..max_desc_len])
            } else {
                cmd.description.clone()
            };

            let content = Line::from(vec![
                Span::styled(prefix, theme.primary()),
                Span::styled(format!("{} ", COMMAND_ICON), theme.accent()),
                Span::styled(
                    format!("/{}", cmd.name),
                    if is_selected {
                        theme.primary().add_modifier(Modifier::BOLD)
                    } else {
                        theme.text()
                    },
                ),
                Span::styled(" - ", theme.text_muted()),
                Span::styled(
                    desc,
                    if is_selected {
                        theme.text()
                    } else {
                        theme.text_muted()
                    },
                ),
            ]);

            ListItem::new(content)
        })
        .collect();

    let list = List::new(items).highlight_style(theme.selected());
    frame.render_widget(list, inner);
}

// Icons for details dialog
const STATUS_ICON: &str = "●";
const CLOCK_ICON: &str = "🕐";
const USER_ICON: &str = "👤";
const PR_ICON: &str = "🔀";
const ENTITY_ICON: &str = "◆";
const POLICY_ICON: &str = "🛡️";

/// Render the Neo task details dialog
pub fn render_neo_details_dialog(frame: &mut Frame, theme: &Theme, task: &NeoTask) {
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

    let started_on = task
        .created_at
        .as_deref()
        .map(|ts| {
            // Try to parse and format the timestamp nicely
            if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(ts) {
                dt.format("%b %d, %Y, %I:%M:%S %p").to_string()
            } else {
                ts.to_string()
            }
        })
        .unwrap_or_else(|| "Unknown".to_string());

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

    let started_by = task
        .started_by
        .as_ref()
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

// ─────────────────────────────────────────────────────────────
// Slash Commands Management Dialog
// ─────────────────────────────────────────────────────────────

// Icons for slash commands dialog
const BUILTIN_ICON: &str = "🔒";
const CUSTOM_ICON: &str = "✏️";

/// Props for slash commands management dialog
pub struct SlashCommandsDialogProps<'a> {
    pub view: SlashCommandsDialogView,
    pub commands: &'a mut StatefulList<crate::api::NeoSlashCommand>,
    pub selected_detail: Option<&'a crate::api::NeoSlashCommand>,
    pub create_name: &'a TextInput,
    pub create_description: &'a TextInput,
    pub create_prompt: &'a TextEditor,
    pub create_focus: usize,
    pub detail_scroll: &'a mut ScrollViewState,
    pub edit_description: &'a TextInput,
    pub edit_prompt: &'a TextEditor,
    pub edit_focus: usize,
}

/// Render the slash commands management dialog
pub fn render_slash_commands_dialog(
    frame: &mut Frame,
    theme: &Theme,
    props: SlashCommandsDialogProps<'_>,
) {
    let area = centered_rect(80, 80, frame.area());

    // Clear background
    frame.render_widget(Clear, area);

    match props.view {
        SlashCommandsDialogView::List => {
            render_slash_commands_list(frame, theme, area, props.commands);
        }
        SlashCommandsDialogView::Detail => {
            if let Some(cmd) = props.selected_detail {
                render_slash_command_detail(frame, theme, area, cmd, props.detail_scroll);
            }
        }
        SlashCommandsDialogView::Create => {
            render_slash_command_create(
                frame,
                theme,
                area,
                props.create_name,
                props.create_description,
                props.create_prompt,
                props.create_focus,
            );
        }
        SlashCommandsDialogView::Edit => {
            if let Some(cmd) = props.selected_detail {
                render_slash_command_edit(
                    frame,
                    theme,
                    area,
                    cmd,
                    props.edit_description,
                    props.edit_prompt,
                    props.edit_focus,
                );
            }
        }
        SlashCommandsDialogView::ConfirmDelete => {
            if let Some(cmd) = props.selected_detail {
                render_slash_command_delete_confirm(frame, theme, area, cmd);
            }
        }
    }
}

/// Render the list of slash commands
fn render_slash_commands_list(
    frame: &mut Frame,
    theme: &Theme,
    area: Rect,
    commands: &mut StatefulList<crate::api::NeoSlashCommand>,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.border_focused())
        .title(" Slash Commands ")
        .title_style(theme.title());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Split inner area for list and footer
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(3)])
        .split(inner);

    // Get selected index before borrowing items
    let selected_idx = commands.selected_index();

    // Build list items
    let items: Vec<ListItem> = commands
        .items()
        .iter()
        .enumerate()
        .map(|(i, cmd)| {
            let is_selected = selected_idx == Some(i);

            let prefix = if is_selected {
                format!("{} ", symbols::ARROW_RIGHT)
            } else {
                "  ".to_string()
            };

            let type_icon = if cmd.built_in {
                BUILTIN_ICON
            } else {
                CUSTOM_ICON
            };

            // Truncate description if too long
            let max_desc_len = 60;
            let desc = if cmd.description.len() > max_desc_len {
                format!("{}...", &cmd.description[..max_desc_len])
            } else {
                cmd.description.clone()
            };

            let content = Line::from(vec![
                Span::styled(prefix, theme.primary()),
                Span::styled(format!("{} ", type_icon), theme.text_secondary()),
                Span::styled(
                    format!("/{}", cmd.name),
                    if is_selected {
                        theme.primary().add_modifier(Modifier::BOLD)
                    } else {
                        theme.text()
                    },
                ),
                Span::styled(" - ", theme.text_muted()),
                Span::styled(
                    desc,
                    if is_selected {
                        theme.text()
                    } else {
                        theme.text_muted()
                    },
                ),
            ]);

            ListItem::new(content)
        })
        .collect();

    let list = List::new(items).highlight_style(theme.selected());
    frame.render_stateful_widget(list, chunks[0], &mut commands.state);

    // Footer with keyboard hints
    let footer_lines = vec![
        Line::from(vec![
            Span::styled(" ↑↓", theme.key_hint()),
            Span::styled(": navigate | ", theme.text_muted()),
            Span::styled("Enter", theme.key_hint()),
            Span::styled(": view | ", theme.text_muted()),
            Span::styled("n", theme.key_hint()),
            Span::styled(": new | ", theme.text_muted()),
            Span::styled("e", theme.key_hint()),
            Span::styled(": edit | ", theme.text_muted()),
            Span::styled("d", theme.key_hint()),
            Span::styled(": delete | ", theme.text_muted()),
            Span::styled("Esc", theme.key_hint()),
            Span::styled(": close", theme.text_muted()),
        ]),
        Line::from(vec![
            Span::styled(format!(" {} ", BUILTIN_ICON), theme.text_secondary()),
            Span::styled("= built-in | ", theme.text_muted()),
            Span::styled(format!("{} ", CUSTOM_ICON), theme.text_secondary()),
            Span::styled("= custom (editable)", theme.text_muted()),
        ]),
    ];

    let footer = Paragraph::new(footer_lines);
    frame.render_widget(footer, chunks[1]);
}

/// Render command detail view
#[allow(clippy::vec_init_then_push)]
fn render_slash_command_detail(
    frame: &mut Frame,
    theme: &Theme,
    area: Rect,
    cmd: &crate::api::NeoSlashCommand,
    scroll_state: &mut ScrollViewState,
) {
    let type_label = if cmd.built_in { "Built-in" } else { "Custom" };
    let title = format!(" /{} ({}) ", cmd.name, type_label);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.border_focused())
        .title(title)
        .title_style(theme.title());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Split for content and footer
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(2)])
        .split(inner);

    let mut lines: Vec<Line> = Vec::new();

    // Description section
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        " Description",
        theme.subtitle().add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(Span::styled(
        " ─".repeat(20),
        theme.text_muted(),
    )));
    lines.push(Line::from(vec![
        Span::styled("  ", theme.text()),
        Span::styled(&cmd.description, theme.text()),
    ]));

    // Prompt section
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        " Prompt",
        theme.subtitle().add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(Span::styled(
        " ─".repeat(20),
        theme.text_muted(),
    )));

    // Show the prompt content
    for line in cmd.prompt.lines() {
        lines.push(Line::from(vec![
            Span::styled("  ", theme.text()),
            Span::styled(line, theme.text_secondary()),
        ]));
    }

    // Render with scrolling
    let visible_height = chunks[0].height as usize;
    let wrap_options = ratatui::widgets::Wrap { trim: false };
    // Calculate total lines for scrollbar by creating a temporary paragraph
    let total_lines = Paragraph::new(lines.clone())
        .wrap(wrap_options)
        .line_count(chunks[0].width);
    let max_scroll = total_lines.saturating_sub(visible_height);
    let scroll_y = (scroll_state.offset().y as usize).min(max_scroll) as u16;
    // Render the scrolled paragraph
    let content_para = Paragraph::new(lines)
        .wrap(wrap_options)
        .scroll((scroll_y, 0));
    frame.render_widget(content_para, chunks[0]);

    // Scrollbar if needed
    if total_lines > visible_height {
        let scrollbar_area = Rect::new(
            chunks[0].right().saturating_sub(1),
            chunks[0].y,
            1,
            chunks[0].height,
        );

        let thumb_height = ((visible_height * visible_height) / total_lines).max(1);
        let thumb_pos = if max_scroll > 0 {
            (scroll_y as usize * (visible_height - thumb_height)) / max_scroll
        } else {
            0
        };

        for y in 0..chunks[0].height {
            let y_pos = scrollbar_area.y + y;
            let is_thumb = (y as usize) >= thumb_pos && (y as usize) < thumb_pos + thumb_height;
            let symbol = if is_thumb { "█" } else { "░" };
            let style = if is_thumb {
                theme.primary()
            } else {
                theme.text_muted()
            };

            frame
                .buffer_mut()
                .set_string(scrollbar_area.x, y_pos, symbol, style);
        }
    }

    // Footer
    let footer = Line::from(vec![
        Span::styled(" j/k", theme.key_hint()),
        Span::styled(": scroll | ", theme.text_muted()),
        Span::styled("Esc", theme.key_hint()),
        Span::styled(": back to list", theme.text_muted()),
    ]);
    frame.render_widget(Paragraph::new(footer), chunks[1]);
}

/// Render create command form
fn render_slash_command_create(
    frame: &mut Frame,
    theme: &Theme,
    area: Rect,
    name_input: &TextInput,
    description_input: &TextInput,
    prompt_editor: &TextEditor,
    focus_index: usize,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.border_focused())
        .title(" Create Slash Command ")
        .title_style(theme.title());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Layout: name, description, prompt, footer
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Name
            Constraint::Length(3), // Description
            Constraint::Min(10),   // Prompt
            Constraint::Length(2), // Footer
        ])
        .split(inner);

    // Name input
    let name_block = Block::default()
        .borders(Borders::ALL)
        .border_style(if focus_index == 0 {
            theme.border_focused()
        } else {
            theme.border()
        })
        .title(" Name (no leading /) ")
        .title_style(if focus_index == 0 {
            theme.primary()
        } else {
            theme.subtitle()
        });

    let name_inner = name_block.inner(chunks[0]);
    frame.render_widget(name_block, chunks[0]);

    let name_value = name_input.value();
    let name_cursor = name_input.cursor();
    if focus_index == 0 {
        let spans = render_input_with_cursor(name_value, name_cursor, theme);
        frame.render_widget(Paragraph::new(Line::from(spans)), name_inner);
    } else {
        frame.render_widget(Paragraph::new(name_value).style(theme.text()), name_inner);
    }

    // Description input
    let desc_block = Block::default()
        .borders(Borders::ALL)
        .border_style(if focus_index == 1 {
            theme.border_focused()
        } else {
            theme.border()
        })
        .title(" Description ")
        .title_style(if focus_index == 1 {
            theme.primary()
        } else {
            theme.subtitle()
        });

    let desc_inner = desc_block.inner(chunks[1]);
    frame.render_widget(desc_block, chunks[1]);

    let desc_value = description_input.value();
    let desc_cursor = description_input.cursor();
    if focus_index == 1 {
        let spans = render_input_with_cursor(desc_value, desc_cursor, theme);
        frame.render_widget(Paragraph::new(Line::from(spans)), desc_inner);
    } else {
        frame.render_widget(Paragraph::new(desc_value).style(theme.text()), desc_inner);
    }

    // Prompt editor
    let prompt_block = Block::default()
        .borders(Borders::ALL)
        .border_style(if focus_index == 2 {
            theme.border_focused()
        } else {
            theme.border()
        })
        .title(" Prompt (the actual instruction) ")
        .title_style(if focus_index == 2 {
            theme.primary()
        } else {
            theme.subtitle()
        });

    let prompt_inner = prompt_block.inner(chunks[2]);
    frame.render_widget(prompt_block, chunks[2]);

    // Render the prompt editor content
    let prompt_content = prompt_editor.content();
    let prompt_cursor = prompt_editor.cursor();
    let prompt_lines: Vec<Line> = prompt_content
        .lines()
        .enumerate()
        .map(|(line_idx, line)| {
            if focus_index == 2 && line_idx == prompt_cursor.0 {
                // This is the cursor line - show cursor
                let chars: Vec<char> = line.chars().collect();
                let mut spans = Vec::new();
                for (col, ch) in chars.iter().enumerate() {
                    if col == prompt_cursor.1 {
                        spans.push(Span::styled(ch.to_string(), theme.cursor()));
                    } else {
                        spans.push(Span::styled(ch.to_string(), theme.text()));
                    }
                }
                if prompt_cursor.1 >= chars.len() {
                    spans.push(Span::styled(" ", theme.cursor()));
                }
                Line::from(spans)
            } else {
                Line::from(Span::styled(line, theme.text()))
            }
        })
        .collect();

    let prompt_para = Paragraph::new(prompt_lines);
    frame.render_widget(prompt_para, prompt_inner);

    // Footer
    let footer = Line::from(vec![
        Span::styled(" Tab", theme.key_hint()),
        Span::styled(": next field | ", theme.text_muted()),
        Span::styled("Ctrl+S", theme.key_hint()),
        Span::styled(": create | ", theme.text_muted()),
        Span::styled("Esc", theme.key_hint()),
        Span::styled(": cancel", theme.text_muted()),
    ]);
    frame.render_widget(Paragraph::new(footer), chunks[3]);
}

/// Helper to render input with cursor
fn render_input_with_cursor(input: &str, cursor_pos: usize, theme: &Theme) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let chars: Vec<char> = input.chars().collect();

    for (i, ch) in chars.iter().enumerate() {
        if i == cursor_pos {
            spans.push(Span::styled(ch.to_string(), theme.cursor()));
        } else {
            spans.push(Span::styled(ch.to_string(), theme.text()));
        }
    }

    if cursor_pos >= chars.len() {
        spans.push(Span::styled(" ".to_string(), theme.cursor()));
    }

    spans
}

/// Render edit command form
fn render_slash_command_edit(
    frame: &mut Frame,
    theme: &Theme,
    area: Rect,
    cmd: &crate::api::NeoSlashCommand,
    description_input: &TextInput,
    prompt_editor: &TextEditor,
    focus_index: usize,
) {
    let title = format!(" Edit /{} ", cmd.name);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.border_focused())
        .title(title)
        .title_style(theme.title());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Layout: name (read-only), description, prompt, footer
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Name (read-only display)
            Constraint::Length(3), // Description
            Constraint::Min(10),   // Prompt
            Constraint::Length(2), // Footer
        ])
        .split(inner);

    // Name display (read-only)
    let name_block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.border())
        .title(" Name (read-only) ")
        .title_style(theme.text_muted());

    let name_inner = name_block.inner(chunks[0]);
    frame.render_widget(name_block, chunks[0]);
    frame.render_widget(
        Paragraph::new(format!("/{}", cmd.name)).style(theme.text_muted()),
        name_inner,
    );

    // Description input
    let desc_block = Block::default()
        .borders(Borders::ALL)
        .border_style(if focus_index == 0 {
            theme.border_focused()
        } else {
            theme.border()
        })
        .title(" Description ")
        .title_style(if focus_index == 0 {
            theme.primary()
        } else {
            theme.subtitle()
        });

    let desc_inner = desc_block.inner(chunks[1]);
    frame.render_widget(desc_block, chunks[1]);

    let desc_value = description_input.value();
    let desc_cursor = description_input.cursor();
    if focus_index == 0 {
        let spans = render_input_with_cursor(desc_value, desc_cursor, theme);
        frame.render_widget(Paragraph::new(Line::from(spans)), desc_inner);
    } else {
        frame.render_widget(Paragraph::new(desc_value).style(theme.text()), desc_inner);
    }

    // Prompt editor
    let prompt_block = Block::default()
        .borders(Borders::ALL)
        .border_style(if focus_index == 1 {
            theme.border_focused()
        } else {
            theme.border()
        })
        .title(" Prompt (the actual instruction) ")
        .title_style(if focus_index == 1 {
            theme.primary()
        } else {
            theme.subtitle()
        });

    let prompt_inner = prompt_block.inner(chunks[2]);
    frame.render_widget(prompt_block, chunks[2]);

    // Render the prompt editor content
    let prompt_content = prompt_editor.content();
    let prompt_cursor = prompt_editor.cursor();
    let prompt_lines: Vec<Line> = prompt_content
        .lines()
        .enumerate()
        .map(|(line_idx, line)| {
            if focus_index == 1 && line_idx == prompt_cursor.0 {
                // This is the cursor line - show cursor
                let chars: Vec<char> = line.chars().collect();
                let mut spans = Vec::new();
                for (col, ch) in chars.iter().enumerate() {
                    if col == prompt_cursor.1 {
                        spans.push(Span::styled(ch.to_string(), theme.cursor()));
                    } else {
                        spans.push(Span::styled(ch.to_string(), theme.text()));
                    }
                }
                if prompt_cursor.1 >= chars.len() {
                    spans.push(Span::styled(" ", theme.cursor()));
                }
                Line::from(spans)
            } else {
                Line::from(Span::styled(line, theme.text()))
            }
        })
        .collect();

    let prompt_para = Paragraph::new(prompt_lines);
    frame.render_widget(prompt_para, prompt_inner);

    // Footer
    let footer = Line::from(vec![
        Span::styled(" Tab", theme.key_hint()),
        Span::styled(": next field | ", theme.text_muted()),
        Span::styled("Ctrl+S", theme.key_hint()),
        Span::styled(": save | ", theme.text_muted()),
        Span::styled("Esc", theme.key_hint()),
        Span::styled(": cancel", theme.text_muted()),
    ]);
    frame.render_widget(Paragraph::new(footer), chunks[3]);
}

/// Render delete confirmation dialog
fn render_slash_command_delete_confirm(
    frame: &mut Frame,
    theme: &Theme,
    area: Rect,
    cmd: &crate::api::NeoSlashCommand,
) {
    // Smaller centered area for confirmation
    let confirm_area = centered_rect(50, 30, area);

    frame.render_widget(Clear, confirm_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.warning())
        .title(" Confirm Delete ")
        .title_style(theme.warning());

    let inner = block.inner(confirm_area);
    frame.render_widget(block, confirm_area);

    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            " Are you sure you want to delete this command?",
            theme.text(),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  /", theme.primary()),
            Span::styled(&cmd.name, theme.primary().add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            format!("  {}", &cmd.description),
            theme.text_muted(),
        )),
        Line::from(""),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Press ", theme.text_muted()),
            Span::styled("y", theme.key_hint()),
            Span::styled(" to confirm, ", theme.text_muted()),
            Span::styled("n", theme.key_hint()),
            Span::styled(" or ", theme.text_muted()),
            Span::styled("Esc", theme.key_hint()),
            Span::styled(" to cancel", theme.text_muted()),
        ]),
    ];

    let para = Paragraph::new(lines);
    frame.render_widget(para, inner);
}
