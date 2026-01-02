//! ESC (Environments, Secrets, Configs) view rendering

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    prelude::*,
    text::{Line, Span},
    widgets::{
        Block, Borders, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
    },
};
use tui_scrollview::{ScrollView, ScrollViewState};

use crate::api::EscEnvironmentSummary;
use crate::app::EscPane;
use crate::components::StatefulList;
use crate::theme::{symbols, Theme};

/// Props for rendering the ESC view
pub struct EscViewProps<'a> {
    pub environments: &'a mut StatefulList<EscEnvironmentSummary>,
    pub selected_env_yaml: Option<&'a str>,
    /// Cached syntax-highlighted lines for YAML (avoids re-highlighting on every frame)
    pub selected_env_yaml_highlighted: Option<&'a Vec<Line<'static>>>,
    pub selected_env_values: Option<&'a serde_json::Value>,
    /// Cached syntax-highlighted lines for resolved values (avoids re-highlighting on every frame)
    pub selected_env_values_highlighted: Option<&'a Vec<Line<'static>>>,
    pub focused_pane: EscPane,
    pub definition_scroll: &'a mut ScrollViewState,
    pub values_scroll: &'a mut ScrollViewState,
}

/// Builder for rendering scrollable panes
struct ScrollablePaneBuilder<'a> {
    title: &'a str,
    content: Option<String>,
    /// Pre-computed highlighted lines (if available, skips highlighting)
    highlighted_lines: Option<&'a Vec<Line<'static>>>,
    is_focused: bool,
    scroll_state: &'a mut ScrollViewState,
    hint: &'a str,
    use_syntax_highlight: bool,
}

impl<'a> ScrollablePaneBuilder<'a> {
    fn new(title: &'a str, scroll_state: &'a mut ScrollViewState) -> Self {
        Self {
            title,
            content: None,
            highlighted_lines: None,
            is_focused: false,
            scroll_state,
            hint: "",
            use_syntax_highlight: false,
        }
    }

    fn content(mut self, content: Option<String>) -> Self {
        self.content = content;
        self
    }

    /// Use pre-computed highlighted lines (skips syntax highlighting)
    fn highlighted(mut self, lines: Option<&'a Vec<Line<'static>>>) -> Self {
        self.highlighted_lines = lines;
        self
    }

    fn focused(mut self, is_focused: bool) -> Self {
        self.is_focused = is_focused;
        self
    }

    fn hint(mut self, hint: &'a str) -> Self {
        self.hint = hint;
        self
    }

    fn syntax_highlight(mut self, enable: bool) -> Self {
        self.use_syntax_highlight = enable;
        self
    }

    fn render(self, frame: &mut Frame, theme: &Theme, area: Rect) {
        render_scrollable_pane_impl(
            frame,
            theme,
            area,
            self.title,
            self.content,
            self.highlighted_lines,
            self.is_focused,
            self.scroll_state,
            self.hint,
            self.use_syntax_highlight,
        );
    }
}

/// Props for rendering environment details
struct EnvironmentDetailsProps<'a> {
    selected: Option<&'a EscEnvironmentSummary>,
    yaml: Option<&'a str>,
    yaml_highlighted: Option<&'a Vec<Line<'static>>>,
    values: Option<&'a serde_json::Value>,
    values_highlighted: Option<&'a Vec<Line<'static>>>,
    focused_pane: EscPane,
    definition_scroll: &'a mut ScrollViewState,
    values_scroll: &'a mut ScrollViewState,
}

/// Render the ESC environments view
pub fn render_esc_view(frame: &mut Frame, theme: &Theme, area: Rect, props: EscViewProps<'_>) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    render_environments_list(frame, theme, chunks[0], props.environments);
    render_environment_details(
        frame,
        theme,
        chunks[1],
        EnvironmentDetailsProps {
            selected: props.environments.selected(),
            yaml: props.selected_env_yaml,
            yaml_highlighted: props.selected_env_yaml_highlighted,
            values: props.selected_env_values,
            values_highlighted: props.selected_env_values_highlighted,
            focused_pane: props.focused_pane,
            definition_scroll: props.definition_scroll,
            values_scroll: props.values_scroll,
        },
    );
}

fn render_environments_list(
    frame: &mut Frame,
    theme: &Theme,
    area: Rect,
    environments: &mut StatefulList<EscEnvironmentSummary>,
) {
    // Get values before borrowing items
    let selected_idx = environments.selected_index();
    let is_empty = environments.is_empty();

    // Collect item data to owned strings
    let item_data: Vec<(String, String)> = environments
        .items()
        .iter()
        .map(|env| (env.project.clone(), env.name.clone()))
        .collect();

    let items: Vec<ListItem> = item_data
        .iter()
        .enumerate()
        .map(|(i, (project, name))| {
            let is_selected = selected_idx == Some(i);

            let content = Line::from(vec![
                Span::styled(
                    if is_selected {
                        format!("{} ", symbols::ARROW_RIGHT)
                    } else {
                        "  ".to_string()
                    },
                    theme.secondary(),
                ),
                Span::styled(project.as_str(), theme.text()),
                Span::styled("/", theme.text_muted()),
                Span::styled(name.as_str(), theme.highlight()),
            ]);

            ListItem::new(content)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(if is_empty {
                    theme.border()
                } else {
                    theme.border_focused()
                })
                .title(" ESC Environments ")
                .title_style(theme.title()),
        )
        .highlight_style(theme.selected())
        .highlight_symbol("");

    frame.render_stateful_widget(list, area, &mut environments.state);
}

/// Extract only the actual values from the ESC resolved values response,
/// filtering out metadata like executionContext, schema, exprs, and trace info.
///
/// The API response structure is:
/// {
///   "exprs": { ... },           // Expression definitions - skip
///   "properties": {             // Contains the actual values we want
///     "pulumiConfig": { "value": { "key": { "value": "actual_value", "trace": ... } } },
///     "myValues": { "value": { ... } }
///   },
///   "schema": { ... },          // JSON schema - skip
///   "executionContext": { ... } // Metadata - skip
/// }
pub fn extract_values(values: &serde_json::Value) -> serde_json::Value {
    if let Some(obj) = values.as_object() {
        // If there's a "properties" key, extract values from it
        // This is the main structure returned by the ESC open API
        if let Some(properties) = obj.get("properties") {
            if let Some(prop_obj) = properties.as_object() {
                let mut result = serde_json::Map::new();
                for (key, val) in prop_obj {
                    // Skip executionContext metadata
                    if key == "executionContext" {
                        continue;
                    }
                    // Extract the actual value, recursively processing nested structures
                    result.insert(key.clone(), extract_property_value(val));
                }
                if !result.is_empty() {
                    return serde_json::Value::Object(result);
                }
            }
        }

        // If there's a "values" key, use that (some API responses use this)
        if let Some(inner_values) = obj.get("values") {
            return filter_trace_info(inner_values);
        }

        // Fallback: filter out known metadata keys from top level
        let mut result = serde_json::Map::new();
        for (key, val) in obj {
            // Skip all metadata keys
            if key == "executionContext" || key == "schema" || key == "exprs" {
                continue;
            }
            result.insert(key.clone(), filter_trace_info(val));
        }
        return serde_json::Value::Object(result);
    }

    filter_trace_info(values)
}

/// Extract the actual value from a property wrapper.
/// Properties are structured as: { "value": <actual_value>, "trace": { ... }, "secret": bool }
/// The actual_value can itself be an object with nested properties.
fn extract_property_value(prop: &serde_json::Value) -> serde_json::Value {
    if let Some(obj) = prop.as_object() {
        // Check if this is a property wrapper with "value" key
        if let Some(inner_value) = obj.get("value") {
            // Recursively process the inner value (secrets are shown as-is)
            return extract_property_value(inner_value);
        }

        // It's a regular object, process each key
        let mut result = serde_json::Map::new();
        for (key, val) in obj {
            // Skip trace info
            if key == "trace" || key == "secret" {
                continue;
            }
            result.insert(key.clone(), extract_property_value(val));
        }
        return serde_json::Value::Object(result);
    }

    // For arrays, process each element
    if let Some(arr) = prop.as_array() {
        return serde_json::Value::Array(arr.iter().map(extract_property_value).collect());
    }

    // For primitives, return as-is
    prop.clone()
}

/// Recursively filter out trace information from values
fn filter_trace_info(value: &serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(obj) => {
            let mut result = serde_json::Map::new();
            for (key, val) in obj {
                // Skip trace-related keys
                if key == "trace"
                    || key == "def"
                    || key == "begin"
                    || key == "end"
                    || key == "byte"
                    || key == "column"
                    || key == "line"
                    || key == "environment"
                {
                    continue;
                }
                // If this is a "value" wrapper with trace, extract just the value
                if key == "value" && obj.contains_key("trace") {
                    return filter_trace_info(val);
                }
                // For "name" objects that have nested value structure
                if let Some(inner_obj) = val.as_object() {
                    if inner_obj.contains_key("value") && inner_obj.contains_key("trace") {
                        if let Some(inner_value) = inner_obj.get("value") {
                            result.insert(key.clone(), filter_trace_info(inner_value));
                            continue;
                        }
                    }
                }
                result.insert(key.clone(), filter_trace_info(val));
            }
            serde_json::Value::Object(result)
        }
        serde_json::Value::Array(arr) => {
            serde_json::Value::Array(arr.iter().map(filter_trace_info).collect())
        }
        _ => value.clone(),
    }
}

/// Convert JSON value to YAML string
pub fn json_to_yaml(value: &serde_json::Value) -> String {
    // Use serde_yaml if available, otherwise format manually
    match serde_json::to_value(value) {
        Ok(v) => format_as_yaml(&v, 0),
        Err(_) => "Error converting to YAML".to_string(),
    }
}

/// Format a JSON value as YAML-like string
fn format_as_yaml(value: &serde_json::Value, indent: usize) -> String {
    let indent_str = "  ".repeat(indent);
    match value {
        serde_json::Value::Object(obj) => {
            if obj.is_empty() {
                return "{}".to_string();
            }
            let mut lines = Vec::new();
            for (key, val) in obj {
                match val {
                    serde_json::Value::Object(_) | serde_json::Value::Array(_) => {
                        let nested = format_as_yaml(val, indent + 1);
                        if nested.starts_with('{') || nested.starts_with('[') {
                            lines.push(format!("{}{}: {}", indent_str, key, nested));
                        } else {
                            lines.push(format!("{}{}:", indent_str, key));
                            lines.push(nested);
                        }
                    }
                    _ => {
                        lines.push(format!("{}{}: {}", indent_str, key, format_scalar(val)));
                    }
                }
            }
            lines.join("\n")
        }
        serde_json::Value::Array(arr) => {
            if arr.is_empty() {
                return "[]".to_string();
            }
            let mut lines = Vec::new();
            for item in arr {
                match item {
                    serde_json::Value::Object(_) | serde_json::Value::Array(_) => {
                        let nested = format_as_yaml(item, indent + 1);
                        lines.push(format!("{}- ", indent_str));
                        lines.push(nested);
                    }
                    _ => {
                        lines.push(format!("{}- {}", indent_str, format_scalar(item)));
                    }
                }
            }
            lines.join("\n")
        }
        _ => format!("{}{}", indent_str, format_scalar(value)),
    }
}

/// Format a scalar JSON value for YAML output
fn format_scalar(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => "null".to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::String(s) => {
            // Check if string needs quoting (contains special chars or looks like a number)
            if s.contains(':')
                || s.contains('#')
                || s.contains('\n')
                || s.starts_with(' ')
                || s.ends_with(' ')
                || s == "true"
                || s == "false"
                || s == "null"
                || s.parse::<f64>().is_ok()
            {
                format!("\"{}\"", s.replace('"', "\\\""))
            } else if s.is_empty() {
                "\"\"".to_string()
            } else {
                s.clone()
            }
        }
        _ => value.to_string(),
    }
}

fn render_environment_details(
    frame: &mut Frame,
    theme: &Theme,
    area: Rect,
    props: EnvironmentDetailsProps<'_>,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(8), Constraint::Min(10)])
        .split(area);

    // Environment info
    let info_block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.border())
        .title(" Environment Details ")
        .title_style(theme.subtitle());

    let info_inner = info_block.inner(chunks[0]);
    frame.render_widget(info_block, chunks[0]);

    match props.selected {
        Some(env) => {
            let info_lines = vec![
                Line::from(vec![
                    Span::styled("Organization: ", theme.text_secondary()),
                    Span::styled(&env.organization, theme.text()),
                ]),
                Line::from(vec![
                    Span::styled("Project:      ", theme.text_secondary()),
                    Span::styled(&env.project, theme.primary()),
                ]),
                Line::from(vec![
                    Span::styled("Environment:  ", theme.text_secondary()),
                    Span::styled(&env.name, theme.highlight()),
                ]),
                Line::from(vec![
                    Span::styled("Created:      ", theme.text_secondary()),
                    Span::styled(env.created.as_deref().unwrap_or("Unknown"), theme.text()),
                ]),
                Line::from(vec![
                    Span::styled("Modified:     ", theme.text_secondary()),
                    Span::styled(env.modified.as_deref().unwrap_or("Unknown"), theme.text()),
                ]),
            ];

            let info_para = Paragraph::new(info_lines);
            frame.render_widget(info_para, info_inner);
        }
        None => {
            let empty = Paragraph::new("Select an environment to view details")
                .style(theme.text_muted())
                .alignment(Alignment::Center);
            frame.render_widget(empty, info_inner);
        }
    }

    // YAML / Values tabs
    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[1]);

    // YAML definition pane
    let is_definition_focused = props.focused_pane == EscPane::Definition;
    let definition_hint = if props.selected.is_some() {
        "Press Enter to load definition"
    } else {
        "Select an environment"
    };
    ScrollablePaneBuilder::new(" Definition (YAML) ", props.definition_scroll)
        .content(props.yaml.map(|s| s.to_string()))
        .highlighted(props.yaml_highlighted) // Use cached highlighted lines
        .focused(is_definition_focused)
        .hint(definition_hint)
        .syntax_highlight(true)
        .render(frame, theme, content_chunks[0]);

    // Resolved values pane
    let is_values_focused = props.focused_pane == EscPane::ResolvedValues;
    // Only compute values_content if we don't have cached highlighted lines
    let values_content = if props.values_highlighted.is_none() {
        props.values.map(|v| {
            let filtered = extract_values(v);
            json_to_yaml(&filtered)
        })
    } else {
        None
    };
    let values_hint = if props.selected.is_some() {
        "Press 'o' to open & resolve"
    } else {
        "Select an environment"
    };
    ScrollablePaneBuilder::new(" Resolved Values ", props.values_scroll)
        .content(values_content)
        .highlighted(props.values_highlighted) // Use cached highlighted lines
        .focused(is_values_focused)
        .hint(values_hint)
        .syntax_highlight(true)
        .render(frame, theme, content_chunks[1]);
}

/// Internal implementation for rendering scrollable panes
#[allow(clippy::too_many_arguments)]
fn render_scrollable_pane_impl(
    frame: &mut Frame,
    theme: &Theme,
    area: Rect,
    title: &str,
    content: Option<String>,
    pre_highlighted: Option<&Vec<Line<'static>>>,
    is_focused: bool,
    scroll_state: &mut ScrollViewState,
    hint: &str,
    use_syntax_highlight: bool,
) {
    use super::syntax::highlight_yaml;
    use ratatui::text::Text;

    let border_style = if is_focused {
        theme.border_focused()
    } else {
        theme.border()
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(title)
        .title_style(if is_focused {
            theme.title()
        } else {
            theme.subtitle()
        });

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Use pre-highlighted lines if available, otherwise fall back to content
    let has_content = pre_highlighted.is_some() || content.is_some();

    if has_content {
        // Use cached highlighted lines if available, otherwise compute them
        let highlighted_lines: Vec<Line<'static>> = if let Some(cached) = pre_highlighted {
            cached.clone()
        } else if let Some(ref text) = content {
            if use_syntax_highlight {
                highlight_yaml(text)
            } else {
                text.lines().map(|l| Line::from(l.to_string())).collect()
            }
        } else {
            Vec::new()
        };

        let content_height = highlighted_lines.len() as u16;
        let view_height = inner.height;

        // Create scrollable content
        let mut scroll_view = ScrollView::new(Size::new(
            inner.width.saturating_sub(1),
            content_height.max(view_height),
        ));

        // Render content into scroll view with syntax highlighting
        let content_text = Text::from(highlighted_lines);
        let content_para = Paragraph::new(content_text);
        scroll_view.render_widget(
            content_para,
            Rect::new(
                0,
                0,
                inner.width.saturating_sub(1),
                content_height.max(view_height),
            ),
        );

        // Render scroll view
        frame.render_stateful_widget(scroll_view, inner, scroll_state);

        // Render scrollbar if content exceeds view height
        if content_height > view_height {
            let scrollbar_area = Rect::new(
                inner.x + inner.width.saturating_sub(1),
                inner.y,
                1,
                inner.height,
            );

            let scroll_position = scroll_state.offset().y as usize;
            let mut scrollbar_state =
                ScrollbarState::new(content_height.saturating_sub(view_height) as usize)
                    .position(scroll_position);

            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("▲"))
                .end_symbol(Some("▼"))
                .track_symbol(Some("│"))
                .thumb_symbol("█");

            frame.render_stateful_widget(scrollbar, scrollbar_area, &mut scrollbar_state);
        }
    } else {
        let empty = Paragraph::new(hint)
            .style(theme.text_muted())
            .alignment(Alignment::Center);
        frame.render_widget(empty, inner);
    }
}

/// Render the YAML editor dialog
pub fn render_esc_editor(
    frame: &mut Frame,
    theme: &Theme,
    editor: &crate::components::TextEditor,
    env_name: &str,
) {
    use super::syntax::highlight_yaml;
    use ratatui::text::Text;
    use ratatui::widgets::Clear;

    // Create a large centered dialog (90% width, 85% height)
    let area = frame.area();
    let dialog_width = (area.width as f32 * 0.9) as u16;
    let dialog_height = (area.height as f32 * 0.85) as u16;
    let dialog_x = (area.width.saturating_sub(dialog_width)) / 2;
    let dialog_y = (area.height.saturating_sub(dialog_height)) / 2;
    let dialog_area = Rect::new(dialog_x, dialog_y, dialog_width, dialog_height);

    // Clear background
    frame.render_widget(Clear, dialog_area);

    // Main block with title and instructions
    let modified_indicator = if editor.is_modified() {
        " [modified]"
    } else {
        ""
    };
    let title = format!(" Edit: {} {}", env_name, modified_indicator);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.border_focused())
        .title(title)
        .title_style(theme.title())
        .title_bottom(Line::from(vec![
            Span::styled(" Esc", theme.key_hint()),
            Span::styled(": Save & Close | ", theme.key_desc()),
            Span::styled("Ctrl+C", theme.key_hint()),
            Span::styled(": Cancel | ", theme.key_desc()),
            Span::styled("Tab", theme.key_hint()),
            Span::styled(": Indent | ", theme.key_desc()),
            Span::styled("Ctrl+D", theme.key_hint()),
            Span::styled(": Delete line ", theme.key_desc()),
        ]));

    let inner = block.inner(dialog_area);
    frame.render_widget(block, dialog_area);

    // Split inner area: line numbers | editor content | scrollbar
    let line_number_width = 4u16; // "999 " format
    let editor_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(line_number_width),
            Constraint::Min(10),
            Constraint::Length(1), // scrollbar
        ])
        .split(inner);

    let line_num_area = editor_chunks[0];
    let editor_area = editor_chunks[1];
    let scrollbar_area = editor_chunks[2];

    let visible_height = editor_area.height as usize;
    let scroll_offset = editor.scroll_offset();
    let (cursor_row, cursor_col) = editor.cursor();
    let lines = editor.lines();
    let total_lines = lines.len();

    // Render line numbers
    let line_numbers: Vec<Line> = (scroll_offset..scroll_offset + visible_height)
        .map(|i| {
            if i < total_lines {
                let style = if i == cursor_row {
                    theme.highlight()
                } else {
                    theme.text_muted()
                };
                Line::from(Span::styled(format!("{:>3} ", i + 1), style))
            } else {
                Line::from(Span::styled("    ", theme.text_muted()))
            }
        })
        .collect();

    let line_num_para = Paragraph::new(line_numbers);
    frame.render_widget(line_num_para, line_num_area);

    // Render editor content with syntax highlighting
    let visible_lines: Vec<&str> = lines
        .iter()
        .skip(scroll_offset)
        .take(visible_height)
        .map(|s| s.as_str())
        .collect();

    // Join visible lines for syntax highlighting
    let visible_content = visible_lines.join("\n");
    let mut highlighted_lines = highlight_yaml(&visible_content);

    // Pad to fill visible area
    while highlighted_lines.len() < visible_height {
        highlighted_lines.push(Line::from(""));
    }

    // Render content
    let content_text = Text::from(highlighted_lines);
    let content_para = Paragraph::new(content_text);
    frame.render_widget(content_para, editor_area);

    // Render cursor
    let cursor_visible_row = cursor_row.saturating_sub(scroll_offset);
    if cursor_visible_row < visible_height {
        let cursor_x = editor_area.x + cursor_col as u16;
        let cursor_y = editor_area.y + cursor_visible_row as u16;

        // Make sure cursor doesn't go beyond editor area
        if cursor_x < editor_area.x + editor_area.width {
            frame.set_cursor_position((cursor_x, cursor_y));
        }
    }

    // Render scrollbar if content exceeds view
    if total_lines > visible_height {
        let mut scrollbar_state =
            ScrollbarState::new(total_lines.saturating_sub(visible_height)).position(scroll_offset);

        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("▲"))
            .end_symbol(Some("▼"))
            .track_symbol(Some("│"))
            .thumb_symbol("█");

        frame.render_stateful_widget(scrollbar, scrollbar_area, &mut scrollbar_state);
    }
}
