//! Commands view rendering
//!
//! Renders the Pulumi CLI commands interface with categories,
//! command list, parameter dialogs, and output display.

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    prelude::*,
    text::{Line, Span},
    widgets::{
        Block, Borders, Clear, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation,
        ScrollbarState, Wrap,
    },
};
use tui_scrollview::ScrollViewState;

use crate::commands::{
    commands_by_category, CommandCategory, CommandExecution, CommandExecutionState, ExecutionMode,
    PulumiCommand,
};
use crate::components::{StatefulList, TextInput};
use crate::theme::{symbols, Theme};

/// Props for rendering the commands view
pub struct CommandsViewProps<'a> {
    pub view_state: CommandsViewState,
    pub category_list: &'a mut StatefulList<CommandCategory>,
    pub command_list: &'a mut StatefulList<&'static PulumiCommand>,
    pub current_execution: Option<&'a CommandExecution>,
    pub param_inputs: &'a [TextInput],
    pub param_focus_index: usize,
    pub output_scroll: &'a mut ScrollViewState,
    pub filter_input: &'a TextInput,
    pub is_filtering: bool,
}

/// Props for rendering the sidebar
struct SidebarProps<'a> {
    view_state: CommandsViewState,
    category_list: &'a mut StatefulList<CommandCategory>,
    command_list: &'a mut StatefulList<&'static PulumiCommand>,
    filter_input: &'a TextInput,
    is_filtering: bool,
}

/// State for the commands view
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CommandsViewState {
    /// Browsing categories
    #[default]
    BrowsingCategories,
    /// Browsing commands in a category
    BrowsingCommands,
    /// Showing parameter input dialog
    InputDialog,
    /// Showing confirmation dialog
    ConfirmDialog,
    /// Showing command output
    OutputView,
}

/// Render the commands view
pub fn render_commands_view(
    frame: &mut Frame,
    theme: &Theme,
    area: Rect,
    props: CommandsViewProps<'_>,
) {
    // Main layout: left sidebar for categories/commands, right for details/output
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(area);

    // Left sidebar: categories + commands
    render_sidebar(
        frame,
        theme,
        main_chunks[0],
        SidebarProps {
            view_state: props.view_state,
            category_list: props.category_list,
            command_list: props.command_list,
            filter_input: props.filter_input,
            is_filtering: props.is_filtering,
        },
    );

    // Right panel: command details or output
    render_main_panel(
        frame,
        theme,
        main_chunks[1],
        props.view_state,
        props.command_list.selected().copied(),
        props.current_execution,
        props.output_scroll,
    );

    // Overlay dialogs
    if props.view_state == CommandsViewState::InputDialog {
        if let Some(exec) = props.current_execution {
            render_input_dialog(
                frame,
                theme,
                exec,
                props.param_inputs,
                props.param_focus_index,
            );
        }
    }

    if props.view_state == CommandsViewState::ConfirmDialog {
        if let Some(exec) = props.current_execution {
            render_confirm_dialog(frame, theme, exec);
        }
    }
}

/// Render the left sidebar with categories and commands
fn render_sidebar(frame: &mut Frame, theme: &Theme, area: Rect, props: SidebarProps<'_>) {
    // Split into filter input and lists
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Filter/search
            Constraint::Min(5),    // Lists
        ])
        .split(area);

    // Filter/search input
    render_filter_input(
        frame,
        theme,
        chunks[0],
        props.filter_input,
        props.is_filtering,
    );

    // Categories and commands in a vertical split
    let list_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(chunks[1]);

    // Categories list
    render_categories_list(
        frame,
        theme,
        list_chunks[0],
        props.category_list,
        props.view_state,
    );

    // Commands list
    render_commands_list(
        frame,
        theme,
        list_chunks[1],
        props.command_list,
        props.view_state,
    );
}

/// Render the filter input
fn render_filter_input(
    frame: &mut Frame,
    theme: &Theme,
    area: Rect,
    input: &TextInput,
    is_focused: bool,
) {
    let border_style = if is_focused {
        theme.border_focused()
    } else {
        theme.border()
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(" / Search ")
        .title_style(if is_focused {
            theme.title()
        } else {
            theme.subtitle()
        });

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let value = input.value();
    let display_value = if value.is_empty() && !is_focused {
        "Type / to filter commands...".to_string()
    } else {
        value.to_string()
    };

    let style = if value.is_empty() && !is_focused {
        theme.text_muted()
    } else {
        theme.text()
    };

    let text = Paragraph::new(display_value).style(style);
    frame.render_widget(text, inner);

    // Render cursor if focused
    if is_focused {
        let cursor_x = inner.x + input.cursor() as u16;
        let cursor_y = inner.y;
        if cursor_x < inner.x + inner.width {
            frame.set_cursor_position((cursor_x, cursor_y));
        }
    }
}

/// Render the categories list
fn render_categories_list(
    frame: &mut Frame,
    theme: &Theme,
    area: Rect,
    category_list: &mut StatefulList<CommandCategory>,
    view_state: CommandsViewState,
) {
    let is_focused = view_state == CommandsViewState::BrowsingCategories;
    let selected_idx = category_list.selected_index();

    let items: Vec<ListItem> = category_list
        .items()
        .iter()
        .enumerate()
        .map(|(i, cat)| {
            let is_selected = selected_idx == Some(i);
            let cmd_count = commands_by_category(*cat).len();

            let prefix = if is_selected && is_focused {
                format!("{} ", symbols::ARROW_RIGHT)
            } else {
                "  ".to_string()
            };

            let content = Line::from(vec![
                Span::styled(prefix, theme.primary()),
                Span::styled(cat.icon(), theme.accent()),
                Span::styled(" ", theme.text()),
                Span::styled(
                    cat.title(),
                    if is_selected && is_focused {
                        theme.primary()
                    } else {
                        theme.text()
                    },
                ),
                Span::styled(format!(" ({})", cmd_count), theme.text_muted()),
            ]);

            ListItem::new(content)
        })
        .collect();

    let border_style = if is_focused {
        theme.border_focused()
    } else {
        theme.border()
    };

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(border_style)
                .title(" Categories ")
                .title_style(if is_focused {
                    theme.title()
                } else {
                    theme.subtitle()
                }),
        )
        .highlight_style(theme.selected())
        .highlight_symbol("");

    frame.render_stateful_widget(list, area, &mut category_list.state);
}

/// Render the commands list
fn render_commands_list(
    frame: &mut Frame,
    theme: &Theme,
    area: Rect,
    command_list: &mut StatefulList<&'static PulumiCommand>,
    view_state: CommandsViewState,
) {
    let is_focused = view_state == CommandsViewState::BrowsingCommands;
    let selected_idx = command_list.selected_index();

    let items: Vec<ListItem> = command_list
        .items()
        .iter()
        .enumerate()
        .map(|(i, cmd)| {
            let is_selected = selected_idx == Some(i);

            let prefix = if is_selected && is_focused {
                format!("{} ", symbols::ARROW_RIGHT)
            } else {
                "  ".to_string()
            };

            // Shortcut hint
            let shortcut = cmd
                .shortcut
                .map(|c| format!("[{}] ", c))
                .unwrap_or_default();

            // Execution mode indicator
            let mode_indicator = match cmd.execution_mode {
                ExecutionMode::Streaming => symbols::ARROW_RIGHT,
                ExecutionMode::Quick => symbols::BULLET,
                ExecutionMode::Interactive => symbols::STAR,
            };

            let content = Line::from(vec![
                Span::styled(prefix, theme.primary()),
                Span::styled(shortcut, theme.accent()),
                Span::styled(
                    cmd.name,
                    if is_selected && is_focused {
                        theme.primary()
                    } else {
                        theme.text()
                    },
                ),
                Span::styled(" ", theme.text()),
                Span::styled(mode_indicator, theme.text_muted()),
            ]);

            ListItem::new(content)
        })
        .collect();

    let border_style = if is_focused {
        theme.border_focused()
    } else {
        theme.border()
    };

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(border_style)
                .title(" Commands ")
                .title_style(if is_focused {
                    theme.title()
                } else {
                    theme.subtitle()
                }),
        )
        .highlight_style(theme.selected())
        .highlight_symbol("");

    frame.render_stateful_widget(list, area, &mut command_list.state);
}

/// Render the main panel (details or output)
fn render_main_panel(
    frame: &mut Frame,
    theme: &Theme,
    area: Rect,
    view_state: CommandsViewState,
    selected_command: Option<&'static PulumiCommand>,
    current_execution: Option<&CommandExecution>,
    output_scroll: &mut ScrollViewState,
) {
    // If we're in output view, show the output
    if view_state == CommandsViewState::OutputView {
        if let Some(exec) = current_execution {
            render_output_view(frame, theme, area, exec, output_scroll);
            return;
        }
    }

    // Otherwise, show command details
    render_command_details(frame, theme, area, selected_command);
}

/// Render command details panel
fn render_command_details(
    frame: &mut Frame,
    theme: &Theme,
    area: Rect,
    selected_command: Option<&'static PulumiCommand>,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.border())
        .title(" Command Details ")
        .title_style(theme.subtitle());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    match selected_command {
        Some(cmd) => {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(6), // Header info
                    Constraint::Length(2), // Separator
                    Constraint::Min(5),    // Parameters
                ])
                .split(inner);

            // Command header
            let header_lines = vec![
                Line::from(vec![
                    Span::styled("Command: ", theme.text_secondary()),
                    Span::styled(cmd.display_command(), theme.primary()),
                ]),
                Line::from(vec![
                    Span::styled("Description: ", theme.text_secondary()),
                    Span::styled(cmd.description, theme.text()),
                ]),
                Line::from(vec![
                    Span::styled("Category: ", theme.text_secondary()),
                    Span::styled(cmd.category.title(), theme.accent()),
                ]),
                Line::from(vec![
                    Span::styled("Mode: ", theme.text_secondary()),
                    Span::styled(
                        match cmd.execution_mode {
                            ExecutionMode::Streaming => "Streaming output",
                            ExecutionMode::Quick => "Quick execution",
                            ExecutionMode::Interactive => "Interactive (not supported)",
                        },
                        match cmd.execution_mode {
                            ExecutionMode::Interactive => theme.warning(),
                            _ => theme.text(),
                        },
                    ),
                ]),
            ];
            let header = Paragraph::new(header_lines);
            frame.render_widget(header, chunks[0]);

            // Separator
            let sep = Paragraph::new(Line::from(vec![
                Span::styled(
                    format!("{} Parameters ", symbols::HORIZONTAL.repeat(3)),
                    theme.text_muted(),
                ),
                Span::styled(
                    symbols::HORIZONTAL.repeat(chunks[1].width.saturating_sub(15) as usize),
                    theme.text_muted(),
                ),
            ]));
            frame.render_widget(sep, chunks[1]);

            // Parameters
            if cmd.params.is_empty() {
                let no_params = Paragraph::new("No parameters required")
                    .style(theme.text_muted())
                    .alignment(Alignment::Center);
                frame.render_widget(no_params, chunks[2]);
            } else {
                let param_lines: Vec<Line> = cmd
                    .params
                    .iter()
                    .map(|p| {
                        let flag_str = match (p.short, p.long) {
                            (Some(s), Some(l)) => format!("{}, {}", s, l),
                            (Some(s), None) => s.to_string(),
                            (None, Some(l)) => l.to_string(),
                            (None, None) => "<positional>".to_string(),
                        };

                        let required_str = if p.required { "*" } else { " " };

                        Line::from(vec![
                            Span::styled(required_str, theme.error()),
                            Span::styled(format!("{:<20}", p.name), theme.primary()),
                            Span::styled(format!("{:<15}", flag_str), theme.text_muted()),
                            Span::styled(p.description, theme.text()),
                        ])
                    })
                    .collect();

                let params = Paragraph::new(param_lines);
                frame.render_widget(params, chunks[2]);
            }

            // Hint at bottom
            let hint = if cmd.execution_mode == ExecutionMode::Interactive {
                "This command requires interactive input and cannot run in the TUI"
            } else if cmd.needs_confirmation {
                "Press Enter to configure and run (requires confirmation)"
            } else {
                "Press Enter to configure and run"
            };

            let hint_area = Rect {
                x: inner.x,
                y: inner.y + inner.height.saturating_sub(1),
                width: inner.width,
                height: 1,
            };
            let hint_text = Paragraph::new(hint)
                .style(theme.text_muted())
                .alignment(Alignment::Center);
            frame.render_widget(hint_text, hint_area);
        }
        None => {
            let empty = Paragraph::new("Select a command to view details")
                .style(theme.text_muted())
                .alignment(Alignment::Center);
            frame.render_widget(empty, inner);
        }
    }
}

/// Render the output view for a running/completed command
fn render_output_view(
    frame: &mut Frame,
    theme: &Theme,
    area: Rect,
    execution: &CommandExecution,
    scroll_state: &mut ScrollViewState,
) {
    // Split into header and output
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4), // Header with command info
            Constraint::Min(5),    // Output area
            Constraint::Length(1), // Status bar
        ])
        .split(area);

    // Header
    let status_style = match &execution.state {
        CommandExecutionState::Running => theme.warning(),
        CommandExecutionState::Completed => theme.success(),
        CommandExecutionState::Failed(_) => theme.error(),
        _ => theme.text(),
    };

    let status_text = match &execution.state {
        CommandExecutionState::Running => "Running...".to_string(),
        CommandExecutionState::Completed => {
            format!("Completed (exit: {})", execution.exit_code.unwrap_or(0))
        }
        CommandExecutionState::Failed(e) => format!("Failed: {}", e),
        _ => "".to_string(),
    };

    let header_lines = vec![
        Line::from(vec![
            Span::styled("$ ", theme.primary()),
            Span::styled(execution.display_with_params(), theme.text()),
        ]),
        Line::from(vec![
            Span::styled("Status: ", theme.text_secondary()),
            Span::styled(status_text, status_style),
        ]),
    ];

    let header_block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.border())
        .title(" Command ")
        .title_style(theme.title());

    let header_inner = header_block.inner(chunks[0]);
    frame.render_widget(header_block, chunks[0]);
    frame.render_widget(Paragraph::new(header_lines), header_inner);

    // Output area
    let output_block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.border_focused())
        .title(" Output ")
        .title_style(theme.subtitle());

    let output_inner = output_block.inner(chunks[1]);
    frame.render_widget(output_block, chunks[1]);

    // Render output lines
    let output_lines: Vec<Line> = execution
        .output_lines
        .iter()
        .map(|line| {
            let style = if line.is_error {
                theme.error()
            } else {
                // Color code based on content for Pulumi output
                colorize_pulumi_output(&line.text, theme)
            };
            Line::styled(&line.text, style)
        })
        .collect();

    // Calculate visible area and scroll
    let visible_height = output_inner.height as usize;
    let total_lines = output_lines.len();

    // Get scroll position from state - use the y offset
    let scroll_offset = scroll_state.offset().y as usize;

    // Clamp scroll offset to valid range
    let max_scroll = total_lines.saturating_sub(visible_height);
    let scroll_offset = scroll_offset.min(max_scroll);

    let visible_lines: Vec<Line> = output_lines
        .into_iter()
        .skip(scroll_offset)
        .take(visible_height)
        .collect();

    let output_para = Paragraph::new(visible_lines).wrap(Wrap { trim: false });
    frame.render_widget(output_para, output_inner);

    // Scrollbar
    if total_lines > visible_height {
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("^"))
            .end_symbol(Some("v"));

        let mut scrollbar_state = ScrollbarState::new(total_lines).position(scroll_offset);

        frame.render_stateful_widget(
            scrollbar,
            chunks[1].inner(Margin {
                vertical: 1,
                horizontal: 0,
            }),
            &mut scrollbar_state,
        );
    }

    // Status bar with scroll hints
    let scroll_hint = if total_lines > visible_height {
        format!(
            " | Line {}-{}/{}",
            scroll_offset + 1,
            (scroll_offset + visible_height).min(total_lines),
            total_lines
        )
    } else {
        String::new()
    };

    let status_bar = match &execution.state {
        CommandExecutionState::Running => format!("j/k: scroll | G: bottom{}", scroll_hint),
        CommandExecutionState::Completed | CommandExecutionState::Failed(_) => {
            format!("j/k: scroll | g/G: top/bottom | Esc: close{}", scroll_hint)
        }
        _ => String::new(),
    };
    let status = Paragraph::new(status_bar)
        .style(theme.text_muted())
        .alignment(Alignment::Center);
    frame.render_widget(status, chunks[2]);
}

/// Colorize Pulumi output based on content
fn colorize_pulumi_output(text: &str, theme: &Theme) -> Style {
    let lower = text.to_lowercase();

    if lower.contains("error") || lower.contains("failed") {
        theme.error()
    } else if lower.contains("warning")
        || lower.contains("warn")
        || lower.contains("creating")
        || lower.contains("updating")
    {
        theme.warning()
    } else if lower.contains("created") || lower.contains("updated") || lower.contains("succeeded")
    {
        theme.success()
    } else if lower.contains("deleting") {
        theme.error()
    } else if lower.contains("deleted") {
        theme.text_muted()
    } else if text.starts_with('+') {
        theme.success()
    } else if text.starts_with('-') {
        theme.error()
    } else if text.starts_with('~') {
        theme.warning()
    } else if text.contains("Previewing") || text.contains("Updating") {
        theme.primary()
    } else if text.starts_with("    ") {
        theme.text_secondary()
    } else {
        theme.text()
    }
}

/// Render the parameter input dialog
fn render_input_dialog(
    frame: &mut Frame,
    theme: &Theme,
    execution: &CommandExecution,
    param_inputs: &[TextInput],
    focus_index: usize,
) {
    let area = centered_rect(60, 70, frame.area());
    frame.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.border_focused())
        .title(format!(" Configure: {} ", execution.command.name))
        .title_style(theme.title());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Layout: parameters + buttons
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(5),    // Parameters
            Constraint::Length(3), // Buttons
        ])
        .split(inner);

    // Render parameters
    let params = execution.command.params;
    if params.is_empty() {
        let no_params = Paragraph::new("No parameters to configure")
            .style(theme.text_muted())
            .alignment(Alignment::Center);
        frame.render_widget(no_params, chunks[0]);
    } else {
        // Calculate height per parameter: 1 line label + 3 lines input (border + content + border)
        let param_height = 4u16;
        let total_height = chunks[0].height;
        let max_params = (total_height / param_height) as usize;

        let visible_params = params.len().min(max_params);

        for (i, param) in params.iter().take(visible_params).enumerate() {
            let y_offset = i as u16 * param_height;
            let param_area = Rect {
                x: chunks[0].x,
                y: chunks[0].y + y_offset,
                width: chunks[0].width,
                height: param_height,
            };

            let is_focused = i == focus_index;

            // Label
            let required_marker = if param.required { "*" } else { " " };
            let label = Line::from(vec![
                Span::styled(required_marker, theme.error()),
                Span::styled(
                    param.name,
                    if is_focused {
                        theme.primary()
                    } else {
                        theme.text()
                    },
                ),
                Span::styled(": ", theme.text_muted()),
                Span::styled(param.description, theme.text_muted()),
            ]);
            let label_area = Rect {
                x: param_area.x,
                y: param_area.y,
                width: param_area.width,
                height: 1,
            };
            frame.render_widget(Paragraph::new(label), label_area);

            // Input box - needs height=3 for borders (top + content + bottom)
            let input_area = Rect {
                x: param_area.x + 2,
                y: param_area.y + 1,
                width: param_area.width.saturating_sub(4),
                height: 3,
            };

            let input_style = if is_focused {
                theme.border_focused()
            } else {
                theme.border()
            };

            let input_block = Block::default()
                .borders(Borders::ALL)
                .border_style(input_style);

            let input_inner = input_block.inner(input_area);

            // Render input block
            frame.render_widget(input_block, input_area);

            // Render input value
            if let Some(input) = param_inputs.get(i) {
                let value = input.value();
                let display = if value.is_empty() {
                    param.default.unwrap_or("").to_string()
                } else {
                    value.to_string()
                };

                let style = if value.is_empty() && !is_focused {
                    theme.text_muted()
                } else {
                    theme.text()
                };

                let text = Paragraph::new(display).style(style);
                frame.render_widget(text, input_inner);

                // Cursor
                if is_focused {
                    let cursor_x = input_inner.x + input.cursor() as u16;
                    if cursor_x < input_inner.x + input_inner.width {
                        frame.set_cursor_position((cursor_x, input_inner.y));
                    }
                }
            }
        }
    }

    // Buttons
    let button_text = Line::from(vec![
        Span::styled("[Enter] ", theme.accent()),
        Span::styled("Run  ", theme.text()),
        Span::styled("[Tab] ", theme.accent()),
        Span::styled("Next  ", theme.text()),
        Span::styled("[Esc] ", theme.accent()),
        Span::styled("Cancel", theme.text()),
    ]);
    let buttons = Paragraph::new(button_text).alignment(Alignment::Center);
    frame.render_widget(buttons, chunks[1]);
}

/// Render the confirmation dialog
fn render_confirm_dialog(frame: &mut Frame, theme: &Theme, execution: &CommandExecution) {
    let area = centered_rect(50, 30, frame.area());
    frame.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.warning())
        .title(" Confirm Execution ")
        .title_style(theme.warning());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(3),    // Message
            Constraint::Length(3), // Command preview
            Constraint::Length(2), // Buttons
        ])
        .split(inner);

    // Warning message
    let message = if execution.command.name == "destroy" {
        "This will DESTROY all resources in your stack!\nThis action cannot be undone."
    } else {
        "This command will modify your infrastructure.\nAre you sure you want to continue?"
    };

    let msg_style = if execution.command.name == "destroy" {
        theme.error()
    } else {
        theme.warning()
    };

    let msg = Paragraph::new(message)
        .style(msg_style)
        .alignment(Alignment::Center);
    frame.render_widget(msg, chunks[0]);

    // Command preview
    let preview = Paragraph::new(format!("$ {}", execution.display_with_params()))
        .style(theme.text_muted())
        .alignment(Alignment::Center);
    frame.render_widget(preview, chunks[1]);

    // Buttons
    let button_text = Line::from(vec![
        Span::styled("[y] ", theme.success()),
        Span::styled("Yes  ", theme.text()),
        Span::styled("[n/Esc] ", theme.error()),
        Span::styled("No", theme.text()),
    ]);
    let buttons = Paragraph::new(button_text).alignment(Alignment::Center);
    frame.render_widget(buttons, chunks[2]);
}

/// Create a centered rect for dialogs
fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
