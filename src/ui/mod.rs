//! UI rendering module
//!
//! Contains all view rendering logic for the TUI.

mod dashboard;
mod esc;
mod header;
mod help;
mod logs;
mod neo;
mod stacks;

pub use dashboard::render_dashboard;
pub use esc::render_esc_view;
pub use header::render_header;
pub use help::render_help;
pub use logs::render_logs;
pub use neo::render_neo_view;
pub use stacks::render_stacks_view;

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    prelude::*,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
};

use crate::components::StatefulList;
use crate::theme::{symbols, Theme};

/// Create a centered rect for popups
pub fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
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

/// Render an error popup
pub fn render_error_popup(frame: &mut Frame, theme: &Theme, message: &str) {
    let area = centered_rect(60, 20, frame.area());

    let block = Block::default()
        .title(" Error ")
        .title_style(theme.error())
        .borders(Borders::ALL)
        .border_style(theme.error());

    let paragraph = Paragraph::new(message)
        .style(theme.text())
        .block(block)
        .wrap(ratatui::widgets::Wrap { trim: true });

    // Clear background
    frame.render_widget(ratatui::widgets::Clear, area);
    frame.render_widget(paragraph, area);
}

/// Render a loading indicator
pub fn render_loading(frame: &mut Frame, theme: &Theme, message: &str, spinner_char: &str) {
    let area = centered_rect(40, 10, frame.area());

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.border());

    let text = format!("{} {}", spinner_char, message);
    let paragraph = Paragraph::new(text)
        .style(theme.primary())
        .block(block)
        .alignment(Alignment::Center);

    frame.render_widget(ratatui::widgets::Clear, area);
    frame.render_widget(paragraph, area);
}

/// Create main layout with header, content, and footer
pub fn main_layout(area: Rect) -> (Rect, Rect, Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Header
            Constraint::Min(10),    // Content
            Constraint::Length(1),  // Footer
        ])
        .split(area);

    (chunks[0], chunks[1], chunks[2])
}

/// Render the status bar / footer
pub fn render_footer(frame: &mut Frame, theme: &Theme, area: Rect, hint: &str) {
    let footer = Paragraph::new(hint)
        .style(theme.key_desc())
        .alignment(Alignment::Center);

    frame.render_widget(footer, area);
}

/// Render the organization selector popup
pub fn render_org_selector(
    frame: &mut Frame,
    theme: &Theme,
    org_list: &mut StatefulList<String>,
    current_org: Option<&str>,
) {
    let area = centered_rect(50, 60, frame.area());

    // Clear background
    frame.render_widget(Clear, area);

    // Get values before borrowing items
    let selected_idx = org_list.selected_index();

    // Collect org data to owned values
    let org_data: Vec<String> = org_list.items().iter().cloned().collect();

    let items: Vec<ListItem> = org_data
        .iter()
        .enumerate()
        .map(|(i, org)| {
            let is_selected = selected_idx == Some(i);
            let is_current = current_org == Some(org.as_str());

            let prefix = if is_selected {
                format!("{} ", symbols::ARROW_RIGHT)
            } else {
                "  ".to_string()
            };

            let suffix = if is_current {
                format!(" {}", symbols::CHECK)
            } else {
                String::new()
            };

            let content = Line::from(vec![
                Span::styled(prefix, theme.primary()),
                Span::styled(org.as_str(), if is_current { theme.primary() } else { theme.text() }),
                Span::styled(suffix, theme.success()),
            ]);

            ListItem::new(content)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(theme.border_focused())
                .title(" Select Organization ")
                .title_style(theme.title()),
        )
        .highlight_style(theme.selected())
        .highlight_symbol("");

    frame.render_stateful_widget(list, area, &mut org_list.state);
}
