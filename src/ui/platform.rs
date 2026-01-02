//! Platform view rendering
//!
//! Displays Services, Components (Registry Packages), and Templates in a tabbed view.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect, Size},
    prelude::*,
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Tabs},
};
use tui_scrollview::{ScrollView, ScrollViewState};

use crate::api::{RegistryPackage, RegistryTemplate, Service};
use crate::app::PlatformView;
use crate::components::StatefulList;
use crate::theme::{symbols, Theme};

use super::markdown::render_markdown_content;

/// Props for rendering the platform view
pub struct PlatformViewProps<'a> {
    pub current_view: PlatformView,
    pub services: &'a mut StatefulList<Service>,
    pub packages: &'a mut StatefulList<RegistryPackage>,
    pub templates: &'a mut StatefulList<RegistryTemplate>,
    pub description_scroll_state: &'a mut ScrollViewState,
}

/// Render the platform view with Services, Components, and Templates
pub fn render_platform_view(
    frame: &mut Frame,
    theme: &Theme,
    area: Rect,
    props: PlatformViewProps<'_>,
) {
    // Main layout: tabs at top, content below
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(5)])
        .split(area);

    // Render tabs
    render_platform_tabs(frame, theme, chunks[0], props.current_view);

    // Render content based on current view
    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(chunks[1]);

    match props.current_view {
        PlatformView::Services => {
            render_services_list(frame, theme, content_chunks[0], props.services);
            render_service_details(frame, theme, content_chunks[1], props.services.selected());
        }
        PlatformView::Components => {
            render_packages_list(frame, theme, content_chunks[0], props.packages);
            render_package_details(
                frame,
                theme,
                content_chunks[1],
                props.packages.selected(),
                props.description_scroll_state,
            );
        }
        PlatformView::Templates => {
            render_templates_list(frame, theme, content_chunks[0], props.templates);
            render_template_details(
                frame,
                theme,
                content_chunks[1],
                props.templates.selected(),
                props.description_scroll_state,
            );
        }
    }
}

fn render_platform_tabs(frame: &mut Frame, theme: &Theme, area: Rect, current_view: PlatformView) {
    let titles: Vec<Line> = PlatformView::all()
        .iter()
        .map(|view| {
            let style = if *view == current_view {
                theme.primary()
            } else {
                theme.text_muted()
            };
            Line::from(Span::styled(format!(" {} ", view.title()), style))
        })
        .collect();

    let tabs = Tabs::new(titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(theme.border())
                .title(" Platform ")
                .title_style(theme.title()),
        )
        .select(current_view.index())
        .highlight_style(theme.primary())
        .divider(Span::styled(" | ", theme.text_muted()));

    frame.render_widget(tabs, area);
}

fn render_services_list(
    frame: &mut Frame,
    theme: &Theme,
    area: Rect,
    services: &mut StatefulList<Service>,
) {
    let selected_idx = services.selected_index();
    let is_empty = services.is_empty();

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(if is_empty {
            theme.border()
        } else {
            theme.border_focused()
        })
        .title(" Services ")
        .title_style(theme.subtitle());

    if is_empty {
        // Render empty state message
        let inner = block.inner(area);
        frame.render_widget(block, area);
        let empty = Paragraph::new("No services found")
            .style(theme.text_muted())
            .alignment(Alignment::Center);
        frame.render_widget(empty, inner);
        return;
    }

    // Collect data to owned values to avoid borrow issues
    let service_names: Vec<String> = services.items().iter().map(|s| s.name.clone()).collect();

    let items: Vec<ListItem> = service_names
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let is_selected = selected_idx == Some(i);

            let content = Line::from(vec![
                Span::styled(
                    if is_selected {
                        format!("{} ", symbols::ARROW_RIGHT)
                    } else {
                        "  ".to_string()
                    },
                    theme.primary(),
                ),
                Span::styled(name.as_str(), theme.text()),
            ]);

            ListItem::new(content)
        })
        .collect();

    let list = List::new(items)
        .block(block)
        .highlight_style(theme.selected())
        .highlight_symbol("");

    frame.render_stateful_widget(list, area, &mut services.state);
}

fn render_service_details(
    frame: &mut Frame,
    theme: &Theme,
    area: Rect,
    selected: Option<&Service>,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.border())
        .title(" Service Details ")
        .title_style(theme.subtitle());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    match selected {
        Some(service) => {
            let owner_info = service
                .owner
                .as_ref()
                .map(|o| format!("{}: {}", o.owner_type, o.name))
                .unwrap_or_else(|| "N/A".to_string());

            let lines = vec![
                Line::from(vec![
                    Span::styled("Name:         ", theme.text_secondary()),
                    Span::styled(&service.name, theme.highlight()),
                ]),
                Line::from(vec![
                    Span::styled("Organization: ", theme.text_secondary()),
                    Span::styled(&service.organization_name, theme.text()),
                ]),
                Line::from(vec![
                    Span::styled("Owner:        ", theme.text_secondary()),
                    Span::styled(owner_info, theme.text()),
                ]),
                Line::from(vec![
                    Span::styled("Description:  ", theme.text_secondary()),
                    Span::styled(
                        service.description.as_deref().unwrap_or("No description"),
                        theme.text_muted(),
                    ),
                ]),
                Line::from(vec![
                    Span::styled("Items:        ", theme.text_secondary()),
                    Span::styled(service.item_count(), theme.info()),
                ]),
            ];

            let para = Paragraph::new(lines).wrap(ratatui::widgets::Wrap { trim: true });
            frame.render_widget(para, inner);
        }
        None => {
            let empty = Paragraph::new("Select a service to view details")
                .style(theme.text_muted())
                .alignment(Alignment::Center);
            frame.render_widget(empty, inner);
        }
    }
}

fn render_packages_list(
    frame: &mut Frame,
    theme: &Theme,
    area: Rect,
    packages: &mut StatefulList<RegistryPackage>,
) {
    let selected_idx = packages.selected_index();
    let is_empty = packages.is_empty();

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(if is_empty {
            theme.border()
        } else {
            theme.border_focused()
        })
        .title(" Components (Packages) ")
        .title_style(theme.subtitle());

    if is_empty {
        // Render empty state message
        let inner = block.inner(area);
        frame.render_widget(block, area);
        let empty = Paragraph::new("No components found")
            .style(theme.text_muted())
            .alignment(Alignment::Center);
        frame.render_widget(empty, inner);
        return;
    }

    // Collect data to owned values to avoid borrow issues
    let pkg_data: Vec<(String, String)> = packages
        .items()
        .iter()
        .map(|pkg| {
            (
                pkg.display_name(),
                pkg.version.clone().unwrap_or_else(|| "?".to_string()),
            )
        })
        .collect();

    let items: Vec<ListItem> = pkg_data
        .iter()
        .enumerate()
        .map(|(i, (name, version))| {
            let is_selected = selected_idx == Some(i);

            let content = Line::from(vec![
                Span::styled(
                    if is_selected {
                        format!("{} ", symbols::ARROW_RIGHT)
                    } else {
                        "  ".to_string()
                    },
                    theme.primary(),
                ),
                Span::styled(name.as_str(), theme.text()),
                Span::styled(format!(" v{}", version), theme.text_muted()),
            ]);

            ListItem::new(content)
        })
        .collect();

    let list = List::new(items)
        .block(block)
        .highlight_style(theme.selected())
        .highlight_symbol("");

    frame.render_stateful_widget(list, area, &mut packages.state);
}

fn render_package_details(
    frame: &mut Frame,
    theme: &Theme,
    area: Rect,
    selected: Option<&RegistryPackage>,
    scroll_state: &mut ScrollViewState,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.border())
        .title(" Component Details ")
        .title_style(theme.subtitle());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    match selected {
        Some(pkg) => {
            // Split into metadata (fixed) and description (scrollable)
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(6), Constraint::Min(3)])
                .split(inner);

            // Fixed metadata section
            let metadata_lines = vec![
                Line::from(vec![
                    Span::styled("Name:        ", theme.text_secondary()),
                    Span::styled(pkg.display_name(), theme.highlight()),
                ]),
                Line::from(vec![
                    Span::styled("Full Name:   ", theme.text_secondary()),
                    Span::styled(pkg.full_name(), theme.text()),
                ]),
                Line::from(vec![
                    Span::styled("Version:     ", theme.text_secondary()),
                    Span::styled(pkg.version.as_deref().unwrap_or("N/A"), theme.info()),
                ]),
                Line::from(vec![
                    Span::styled("Publisher:   ", theme.text_secondary()),
                    Span::styled(pkg.publisher.as_deref().unwrap_or("N/A"), theme.text()),
                ]),
                Line::from(vec![
                    Span::styled("Source:      ", theme.text_secondary()),
                    Span::styled(pkg.source.as_deref().unwrap_or("pulumi"), theme.text()),
                ]),
            ];

            let metadata_para = Paragraph::new(metadata_lines);
            frame.render_widget(metadata_para, chunks[0]);

            // Scrollable description section with markdown
            let desc_block = Block::default()
                .borders(Borders::TOP)
                .border_style(theme.border())
                .title(" Description (j/k to scroll) ")
                .title_style(theme.text_muted());

            let desc_inner = desc_block.inner(chunks[1]);
            frame.render_widget(desc_block, chunks[1]);

            // Use readme_content if loaded, otherwise fall back to description
            let description = pkg
                .readme_content
                .as_deref()
                .or(pkg.description.as_deref())
                .unwrap_or("No description available");
            let desc_lines = render_markdown_content(description, theme, "");

            // Calculate content height
            let content_height = desc_lines.len().max(1) as u16;
            let scroll_height = content_height.max(desc_inner.height);

            // Create scrollview
            let mut scroll_view = ScrollView::new(Size::new(desc_inner.width, scroll_height));
            let content_area = Rect::new(0, 0, desc_inner.width, scroll_height);
            let content_para =
                Paragraph::new(desc_lines).wrap(ratatui::widgets::Wrap { trim: false });
            scroll_view.render_widget(content_para, content_area);

            frame.render_stateful_widget(scroll_view, desc_inner, scroll_state);
        }
        None => {
            let empty = Paragraph::new("Select a component to view details")
                .style(theme.text_muted())
                .alignment(Alignment::Center);
            frame.render_widget(empty, inner);
        }
    }
}

fn render_templates_list(
    frame: &mut Frame,
    theme: &Theme,
    area: Rect,
    templates: &mut StatefulList<RegistryTemplate>,
) {
    let selected_idx = templates.selected_index();
    let is_empty = templates.is_empty();

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(if is_empty {
            theme.border()
        } else {
            theme.border_focused()
        })
        .title(" Templates ")
        .title_style(theme.subtitle());

    if is_empty {
        // Render empty state message
        let inner = block.inner(area);
        frame.render_widget(block, area);
        let empty = Paragraph::new("No templates found")
            .style(theme.text_muted())
            .alignment(Alignment::Center);
        frame.render_widget(empty, inner);
        return;
    }

    // Collect data to owned values to avoid borrow issues
    let tmpl_data: Vec<(String, String)> = templates
        .items()
        .iter()
        .map(|tmpl| {
            let lang = tmpl.language.clone().unwrap_or_default();
            let lang_display = if !lang.is_empty() {
                format!(" [{}]", lang)
            } else {
                String::new()
            };
            (tmpl.display(), lang_display)
        })
        .collect();

    let items: Vec<ListItem> = tmpl_data
        .iter()
        .enumerate()
        .map(|(i, (name, lang_display))| {
            let is_selected = selected_idx == Some(i);

            let content = Line::from(vec![
                Span::styled(
                    if is_selected {
                        format!("{} ", symbols::ARROW_RIGHT)
                    } else {
                        "  ".to_string()
                    },
                    theme.primary(),
                ),
                Span::styled(name.as_str(), theme.text()),
                Span::styled(lang_display.as_str(), theme.text_muted()),
            ]);

            ListItem::new(content)
        })
        .collect();

    let list = List::new(items)
        .block(block)
        .highlight_style(theme.selected())
        .highlight_symbol("");

    frame.render_stateful_widget(list, area, &mut templates.state);
}

fn render_template_details(
    frame: &mut Frame,
    theme: &Theme,
    area: Rect,
    selected: Option<&RegistryTemplate>,
    scroll_state: &mut ScrollViewState,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.border())
        .title(" Template Details ")
        .title_style(theme.subtitle());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    match selected {
        Some(tmpl) => {
            // Split into metadata (fixed) and description (scrollable)
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(7), Constraint::Min(3)])
                .split(inner);

            // Fixed metadata section
            let metadata_lines = vec![
                Line::from(vec![
                    Span::styled("Name:        ", theme.text_secondary()),
                    Span::styled(tmpl.display(), theme.highlight()),
                ]),
                Line::from(vec![
                    Span::styled("Full Name:   ", theme.text_secondary()),
                    Span::styled(tmpl.full_name(), theme.text()),
                ]),
                Line::from(vec![
                    Span::styled("Version:     ", theme.text_secondary()),
                    Span::styled(tmpl.version.as_deref().unwrap_or("N/A"), theme.info()),
                ]),
                Line::from(vec![
                    Span::styled("Language:    ", theme.text_secondary()),
                    Span::styled(tmpl.language.as_deref().unwrap_or("N/A"), theme.text()),
                ]),
                Line::from(vec![
                    Span::styled("Runtime:     ", theme.text_secondary()),
                    Span::styled(
                        tmpl.runtime
                            .as_ref()
                            .map(|r| r.name.as_str())
                            .unwrap_or("N/A"),
                        theme.text(),
                    ),
                ]),
                Line::from(vec![
                    Span::styled("Publisher:   ", theme.text_secondary()),
                    Span::styled(tmpl.publisher.as_deref().unwrap_or("N/A"), theme.text()),
                ]),
            ];

            let metadata_para = Paragraph::new(metadata_lines);
            frame.render_widget(metadata_para, chunks[0]);

            // Scrollable description section with markdown
            let desc_block = Block::default()
                .borders(Borders::TOP)
                .border_style(theme.border())
                .title(" Description (j/k to scroll) ")
                .title_style(theme.text_muted());

            let desc_inner = desc_block.inner(chunks[1]);
            frame.render_widget(desc_block, chunks[1]);

            let description = tmpl
                .description
                .as_deref()
                .unwrap_or("No description available");
            let desc_lines = render_markdown_content(description, theme, "");

            // Calculate content height
            let content_height = desc_lines.len().max(1) as u16;
            let scroll_height = content_height.max(desc_inner.height);

            // Create scrollview
            let mut scroll_view = ScrollView::new(Size::new(desc_inner.width, scroll_height));
            let content_area = Rect::new(0, 0, desc_inner.width, scroll_height);
            let content_para =
                Paragraph::new(desc_lines).wrap(ratatui::widgets::Wrap { trim: false });
            scroll_view.render_widget(content_para, content_area);

            frame.render_stateful_widget(scroll_view, desc_inner, scroll_state);
        }
        None => {
            let empty = Paragraph::new("Select a template to view details")
                .style(theme.text_muted())
                .alignment(Alignment::Center);
            frame.render_widget(empty, inner);
        }
    }
}
