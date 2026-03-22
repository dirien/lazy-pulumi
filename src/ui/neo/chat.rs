//! Chat view rendering — message display, input field, thinking indicator

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    prelude::*,
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};
use tui_scrollview::ScrollViewState;

use crate::api::{NeoMessage, NeoMessageType, NeoSlashCommand};
use crate::app::{NeoApprovalMode, NeoPermissionMode, NeoPlanMode, NeoTaskSettings};
use crate::components::TextInput;
use crate::theme::{brand, symbols, Theme};

use super::{
    CommandPickerProps, APPROVAL_ICON, ERROR_ICON, INFO_ICON, RESULT_ICON, THINKING_ICON, TOOL_ICON,
};
use crate::ui::markdown::render_markdown_content;

// Command picker icon
const COMMAND_ICON: &str = "⌘";

/// Props for chat view (internal)
pub(super) struct ChatViewProps<'a> {
    pub messages: &'a [NeoMessage],
    pub input: &'a TextInput,
    pub scroll_state: &'a mut ScrollViewState,
    pub auto_scroll: &'a Arc<AtomicBool>,
    pub is_loading: bool,
    pub spinner_char: &'a str,
    pub command_picker: CommandPickerProps<'a>,
    pub task_settings: NeoTaskSettings,
    pub show_settings: bool,
    pub current_task_plan_mode: bool,
    pub has_loaded_task: bool,
}

pub(super) fn render_chat_view(
    frame: &mut Frame,
    theme: &Theme,
    area: Rect,
    props: ChatViewProps<'_>,
) {
    // Layout: messages area, thinking indicator (if loading), settings bar (if new task),
    // command picker (if showing), input area
    let thinking_height = if props.is_loading { 2 } else { 0 };
    let settings_height: u16 = if props.show_settings { 1 } else { 0 };
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
            Constraint::Length(settings_height),
            Constraint::Length(command_picker_height),
            Constraint::Length(3),
        ])
        .split(area);

    // Plan mode visual indicator — active during composition OR for an existing plan-mode task
    let plan_mode_active = props.current_task_plan_mode
        || (props.show_settings && props.task_settings.plan_mode == crate::app::NeoPlanMode::On);

    // Messages area
    let mut messages_block = Block::default()
        .borders(Borders::ALL)
        .border_style(if plan_mode_active {
            theme.warning()
        } else if props.input.is_focused() {
            theme.border()
        } else {
            theme.border_focused()
        })
        .title(if plan_mode_active {
            " Chat [PLAN] "
        } else {
            " Chat "
        })
        .title_style(if plan_mode_active {
            theme.warning()
        } else {
            theme.subtitle()
        });

    if plan_mode_active {
        messages_block =
            messages_block.border_type(ratatui::widgets::BorderType::LightDoubleDashed);
    }

    let messages_inner = messages_block.inner(chunks[0]);
    frame.render_widget(messages_block, chunks[0]);

    if props.messages.is_empty() {
        if props.is_loading {
            // Just show empty area while loading - the thinking indicator below will show
        } else if props.has_loaded_task {
            // Task is loaded but has no messages yet — shouldn't normally happen
        } else {
            // No task loaded — show welcome or "press Enter" hint
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

    // Task settings bar (shown when composing a new task)
    if props.show_settings {
        render_task_settings_bar(frame, theme, chunks[2], &props.task_settings);
    }

    // Slash command picker (shown above input when typing '/')
    if props.command_picker.show && !props.command_picker.filtered_commands.is_empty() {
        render_command_picker(
            frame,
            theme,
            chunks[3],
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
    let mut input_block = Block::default()
        .borders(Borders::ALL)
        .border_style(if plan_mode_active {
            theme.warning()
        } else if props.input.is_focused() {
            theme.border_focused()
        } else {
            theme.border()
        })
        .title(input_title)
        .title_style(if plan_mode_active {
            theme.warning()
        } else if props.input.is_focused() {
            theme.primary()
        } else {
            theme.subtitle()
        });

    if plan_mode_active {
        input_block = input_block.border_type(ratatui::widgets::BorderType::LightDoubleDashed);
    }

    let input_inner = input_block.inner(chunks[4]);
    frame.render_widget(input_block, chunks[4]);

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

/// Render the task settings bar (approval, permission, plan mode)
fn render_task_settings_bar(
    frame: &mut Frame,
    theme: &Theme,
    area: Rect,
    settings: &NeoTaskSettings,
) {
    let mut spans: Vec<Span> = vec![Span::styled(" ", theme.text_muted())];

    // Approval mode
    spans.push(Span::styled("Approval ", theme.text_muted()));
    let approval_style = if settings.approval_mode != NeoApprovalMode::Default {
        theme.accent().add_modifier(Modifier::BOLD)
    } else {
        theme.text_secondary()
    };
    spans.push(Span::styled(settings.approval_mode.label(), approval_style));
    spans.push(Span::styled(" (", theme.text_muted()));
    spans.push(Span::styled("a", theme.key_hint()));
    spans.push(Span::styled(")  ", theme.text_muted()));

    // Permission mode
    spans.push(Span::styled("Permission ", theme.text_muted()));
    let permission_style = if settings.permission_mode != NeoPermissionMode::Default {
        theme.accent().add_modifier(Modifier::BOLD)
    } else {
        theme.text_secondary()
    };
    spans.push(Span::styled(
        settings.permission_mode.label(),
        permission_style,
    ));
    spans.push(Span::styled(" (", theme.text_muted()));
    spans.push(Span::styled("p", theme.key_hint()));
    spans.push(Span::styled(")  ", theme.text_muted()));

    // Plan mode — extra visual emphasis when active
    spans.push(Span::styled("Plan ", theme.text_muted()));
    let plan_style = if settings.plan_mode == NeoPlanMode::On {
        theme.warning().add_modifier(Modifier::BOLD)
    } else {
        theme.text_secondary()
    };
    spans.push(Span::styled(settings.plan_mode.label(), plan_style));
    spans.push(Span::styled(" (", theme.text_muted()));
    spans.push(Span::styled("m", theme.key_hint()));
    spans.push(Span::styled(")", theme.text_muted()));

    let line = Line::from(spans);
    let para = Paragraph::new(line).style(Style::default().bg(theme.bg_medium));
    frame.render_widget(para, area);
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
