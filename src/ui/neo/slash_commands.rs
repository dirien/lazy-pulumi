//! Slash commands management dialog — list, detail, create, edit, delete

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    prelude::*,
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
};
use tui_scrollview::ScrollViewState;

use crate::api::NeoSlashCommand;
use crate::app::SlashCommandsDialogView;
use crate::components::{StatefulList, TextEditor, TextInput};
use crate::theme::{symbols, Theme};

use super::centered_rect;

// Icons for slash commands dialog
const BUILTIN_ICON: &str = "🔒";
const CUSTOM_ICON: &str = "✏️";

/// Props for slash commands management dialog
pub struct SlashCommandsDialogProps<'a> {
    pub view: SlashCommandsDialogView,
    pub commands: &'a mut StatefulList<NeoSlashCommand>,
    pub selected_detail: Option<&'a NeoSlashCommand>,
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
    commands: &mut StatefulList<NeoSlashCommand>,
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
    cmd: &NeoSlashCommand,
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
    render_prompt_editor(frame, theme, prompt_inner, prompt_editor, focus_index == 2);

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

/// Render edit command form
fn render_slash_command_edit(
    frame: &mut Frame,
    theme: &Theme,
    area: Rect,
    cmd: &NeoSlashCommand,
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
    render_prompt_editor(frame, theme, prompt_inner, prompt_editor, focus_index == 1);

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
    cmd: &NeoSlashCommand,
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

/// Helper to render a prompt editor with optional cursor
fn render_prompt_editor(
    frame: &mut Frame,
    theme: &Theme,
    area: Rect,
    prompt_editor: &TextEditor,
    is_focused: bool,
) {
    let prompt_content = prompt_editor.content();
    let prompt_cursor = prompt_editor.cursor();
    let prompt_lines: Vec<Line> = prompt_content
        .lines()
        .enumerate()
        .map(|(line_idx, line)| {
            if is_focused && line_idx == prompt_cursor.0 {
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
    frame.render_widget(prompt_para, area);
}
