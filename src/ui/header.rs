//! Header rendering

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    prelude::*,
    text::{Line, Span},
    widgets::{Block, Borders, Tabs},
};

use crate::app::Tab;
use crate::theme::Theme;

/// Render the application header with tabs
pub fn render_header(
    frame: &mut Frame,
    theme: &Theme,
    area: Rect,
    active_tab: Tab,
    org: Option<&str>,
) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(40), Constraint::Length(30)])
        .split(area);

    // Tabs
    let tab_titles: Vec<Line> = Tab::all()
        .iter()
        .map(|t| {
            let style = if *t == active_tab {
                theme.tab_active()
            } else {
                theme.tab_inactive()
            };
            Line::from(Span::styled(t.title(), style))
        })
        .collect();

    let tabs = Tabs::new(tab_titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(theme.border())
                .title(" Lazy Pulumi ")
                .title_style(theme.title()),
        )
        .select(active_tab.index())
        .style(theme.text())
        .highlight_style(theme.tab_active())
        .divider(Span::styled(" │ ", theme.text_muted()));

    frame.render_widget(tabs, chunks[0]);

    // Organization info
    let org_text = match org {
        Some(o) => format!(" {} ", o),
        None => " No org selected ".to_string(),
    };

    let org_block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.border())
        .title(" Organization ")
        .title_style(theme.subtitle());

    let org_para = ratatui::widgets::Paragraph::new(org_text)
        .style(theme.primary())
        .block(org_block)
        .alignment(ratatui::layout::Alignment::Center);

    frame.render_widget(org_para, chunks[1]);
}
