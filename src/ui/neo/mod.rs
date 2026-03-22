//! Neo AI agent view rendering

mod chat;
mod details;
mod slash_commands;

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    prelude::*,
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem},
};

use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use tui_scrollview::ScrollViewState;

use crate::api::{NeoSlashCommand, NeoTask};
use crate::app::NeoTaskSettings;
use crate::components::{StatefulList, TextInput};
use crate::theme::{symbols, Theme};

use super::centered_rect;

// Re-export public items expected by src/ui/mod.rs
pub use details::render_neo_details_dialog;
pub use slash_commands::{render_slash_commands_dialog, SlashCommandsDialogProps};

// Tool-related symbols shared across submodules
pub(super) const TOOL_ICON: &str = "🔧";
pub(super) const RESULT_ICON: &str = "📋";
pub(super) const ERROR_ICON: &str = "❌";
pub(super) const APPROVAL_ICON: &str = "❓";
pub(super) const INFO_ICON: &str = "ℹ️";
pub(super) const THINKING_ICON: &str = "🤔";

/// Slash command for the picker
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
    pub messages: &'a [crate::api::NeoMessage],
    pub input: &'a TextInput,
    pub scroll_state: &'a mut ScrollViewState,
    pub auto_scroll: &'a Arc<AtomicBool>,
    pub is_loading: bool,
    pub spinner_char: &'a str,
    pub hide_task_list: bool,
    pub command_picker: CommandPickerProps<'a>,
    pub task_settings: NeoTaskSettings,
    pub show_settings: bool,
    /// Whether the current (already created) task has plan mode enabled
    pub current_task_plan_mode: bool,
    /// Whether a task is currently loaded (current_task_id is Some)
    pub has_loaded_task: bool,
}

/// Render the Neo chat view
pub fn render_neo_view(frame: &mut Frame, theme: &Theme, area: Rect, props: NeoViewProps<'_>) {
    let chat_props = chat::ChatViewProps {
        messages: props.messages,
        input: props.input,
        scroll_state: props.scroll_state,
        auto_scroll: props.auto_scroll,
        is_loading: props.is_loading,
        spinner_char: props.spinner_char,
        command_picker: props.command_picker,
        task_settings: props.task_settings,
        show_settings: props.show_settings,
        current_task_plan_mode: props.current_task_plan_mode,
        has_loaded_task: props.has_loaded_task,
    };

    if props.hide_task_list {
        // Full-width chat when task list is hidden
        chat::render_chat_view(frame, theme, area, chat_props);
    } else {
        // Split view with task list on left
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
            .split(area);

        render_tasks_list(frame, theme, chunks[0], props.tasks);
        chat::render_chat_view(frame, theme, chunks[1], chat_props);
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
