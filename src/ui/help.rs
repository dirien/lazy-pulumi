//! Help popup rendering

use ratatui::{
    prelude::*,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

use crate::theme::Theme;
use crate::ui::centered_rect;

/// Render the help popup
pub fn render_help(frame: &mut Frame, theme: &Theme) {
    let area = centered_rect(70, 80, frame.area());

    // Clear background
    frame.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.border_focused())
        .title(" Help - Keyboard Shortcuts ")
        .title_style(theme.title());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let sections = vec![
        (
            "Global",
            vec![
                ("Tab / Shift+Tab", "Switch between views"),
                ("o", "Select organization"),
                ("l", "View application logs"),
                ("?", "Toggle help"),
                ("q / Ctrl+C", "Quit application"),
                ("r", "Refresh data"),
                ("Esc", "Close popup / Cancel"),
            ],
        ),
        (
            "Navigation",
            vec![
                ("j / ↓", "Move down"),
                ("k / ↑", "Move up"),
                ("g / Home", "Go to first item"),
                ("G / End", "Go to last item"),
                ("Enter", "Select / Confirm"),
            ],
        ),
        (
            "Stacks View",
            vec![
                ("Enter", "View stack details"),
                ("u", "View update history"),
            ],
        ),
        (
            "Environment View",
            vec![
                ("Enter", "Load environment definition"),
                ("o", "Open & resolve environment values"),
            ],
        ),
        (
            "NEO View",
            vec![
                ("n", "Start new task"),
                ("i", "Focus input field"),
                ("Enter", "Send message"),
                ("Esc", "Unfocus input"),
                ("Page Up/Down", "Scroll messages"),
            ],
        ),
    ];

    let mut lines: Vec<Line> = Vec::new();

    for (section_title, keys) in sections {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!(" {} ", section_title),
            theme.title(),
        )));
        lines.push(Line::from(Span::styled(
            " ─".repeat(30),
            theme.text_muted(),
        )));

        for (key, desc) in keys {
            lines.push(Line::from(vec![
                Span::styled(format!("  {:16}", key), theme.key_hint()),
                Span::styled(desc, theme.text_secondary()),
            ]));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        " Press ? or Esc to close ",
        theme.text_muted(),
    )));

    let help_para = Paragraph::new(lines).wrap(ratatui::widgets::Wrap { trim: true });

    frame.render_widget(help_para, inner);
}
