//! Log viewer popup rendering using tui-logger

use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Clear},
};
use tui_logger::{TuiLoggerSmartWidget, TuiWidgetState};

use crate::theme::Theme;
use crate::ui::centered_rect;

/// Render the logs popup using TuiLoggerSmartWidget
pub fn render_logs(frame: &mut Frame, theme: &Theme, logger_state: &TuiWidgetState) {
    let area = centered_rect(90, 85, frame.area());

    // Clear background
    frame.render_widget(Clear, area);

    // Move events from hot buffer to widget buffer
    tui_logger::move_events();

    let title =
        " Logs (h:toggle targets | f:focus | +/-:capture | </>:show | PgUp/Dn:scroll | Esc:close) ";

    // Create the smart widget with target selector and log view
    // Style it to match the Dracula theme
    let logger_widget = TuiLoggerSmartWidget::default()
        // Log level colors matching the theme
        .style_error(theme.error())
        .style_warn(theme.warning())
        .style_info(theme.info())
        .style_debug(theme.text_muted())
        .style_trace(theme.text_muted())
        // Output formatting
        .output_separator('|')
        .output_timestamp(Some("%H:%M:%S".to_string()))
        .output_level(Some(tui_logger::TuiLoggerLevelOutput::Abbreviated))
        .output_target(true)
        .output_file(false)
        .output_line(false)
        // Panel titles
        .title_log(" Messages ")
        .title_target(" Targets ")
        // Border styling - use theme's border color instead of default white
        .border_style(theme.border())
        .border_type(ratatui::widgets::BorderType::Rounded)
        // Highlight style for selected target
        .highlight_style(theme.selected())
        // Connect to state
        .state(logger_state);

    // Create outer block with theme styling
    let outer_block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.border_focused())
        .border_type(ratatui::widgets::BorderType::Rounded)
        .title(title)
        .title_style(theme.title());

    let inner = outer_block.inner(area);
    frame.render_widget(outer_block, area);
    frame.render_widget(logger_widget, inner);
}
