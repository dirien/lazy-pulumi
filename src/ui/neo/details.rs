//! Neo task details dialog rendering

use ratatui::{
    prelude::*,
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

use crate::api::NeoTask;
use crate::theme::{symbols, Theme};

use super::centered_rect;

// Icons for details dialog
const STATUS_ICON: &str = "●";
const CLOCK_ICON: &str = "🕐";
const USER_ICON: &str = "👤";
const PR_ICON: &str = "🔀";
const ENTITY_ICON: &str = "◆";
const POLICY_ICON: &str = "🛡️";

/// Render the Neo task details dialog
pub fn render_neo_details_dialog(frame: &mut Frame, theme: &Theme, task: &NeoTask) {
    let area = centered_rect(25, 70, frame.area());

    // Clear background
    frame.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.border_focused())
        .title(" Task Details ")
        .title_style(theme.title());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut lines: Vec<Line> = Vec::new();

    // Status section
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        " Status",
        theme.subtitle().add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(Span::styled(
        " ─".repeat(18),
        theme.text_muted(),
    )));

    let status = task.status.as_deref().unwrap_or("Unknown");
    let status_style = match status.to_lowercase().as_str() {
        "idle" | "completed" => theme.success(),
        "running" | "in_progress" => theme.warning(),
        "failed" | "error" => theme.error(),
        _ => theme.text_secondary(),
    };

    lines.push(Line::from(vec![
        Span::styled(format!("  {} ", STATUS_ICON), status_style),
        Span::styled(status, status_style),
    ]));

    // Mode section
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        " Mode",
        theme.subtitle().add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(Span::styled(
        " ─".repeat(18),
        theme.text_muted(),
    )));

    if task.plan_mode == Some(true) {
        lines.push(Line::from(vec![
            Span::styled("  📋 ", theme.warning()),
            Span::styled("Plan mode", theme.warning().add_modifier(Modifier::BOLD)),
        ]));
    } else {
        lines.push(Line::from(vec![
            Span::styled("  ⚡ ", theme.text_secondary()),
            Span::styled("Execute mode", theme.text()),
        ]));
    }

    // Started on section
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        " Started on",
        theme.subtitle().add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(Span::styled(
        " ─".repeat(18),
        theme.text_muted(),
    )));

    let started_on = task
        .created_at
        .as_deref()
        .map(|ts| {
            // Try to parse and format the timestamp nicely
            if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(ts) {
                dt.format("%b %d, %Y, %I:%M:%S %p").to_string()
            } else {
                ts.to_string()
            }
        })
        .unwrap_or_else(|| "Unknown".to_string());

    lines.push(Line::from(vec![
        Span::styled(format!("  {} ", CLOCK_ICON), theme.text_secondary()),
        Span::styled(started_on, theme.text()),
    ]));

    // Started by section
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        " Started by",
        theme.subtitle().add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(Span::styled(
        " ─".repeat(18),
        theme.text_muted(),
    )));

    let started_by = task
        .started_by
        .as_ref()
        .and_then(|u| u.login.clone().or(u.name.clone()))
        .unwrap_or_else(|| "Unknown".to_string());

    lines.push(Line::from(vec![
        Span::styled(format!("  {} ", USER_ICON), theme.text_secondary()),
        Span::styled(started_by, theme.text()),
    ]));

    // Linked PRs section
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        " Linked PRs",
        theme.subtitle().add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(Span::styled(
        " ─".repeat(18),
        theme.text_muted(),
    )));

    if task.linked_prs.is_empty() {
        lines.push(Line::from(vec![
            Span::styled(format!("  {} ", PR_ICON), theme.text_muted()),
            Span::styled("No linked PRs", theme.text_muted()),
        ]));
    } else {
        for pr in &task.linked_prs {
            let pr_text = format!(
                "#{} {}",
                pr.number.unwrap_or(0),
                pr.title.as_deref().unwrap_or("Untitled")
            );
            let state = pr.state.as_deref().unwrap_or("");
            let state_style = match state.to_lowercase().as_str() {
                "open" => theme.success(),
                "merged" => theme.primary(),
                "closed" => theme.error(),
                _ => theme.text_muted(),
            };
            lines.push(Line::from(vec![
                Span::styled(format!("  {} ", PR_ICON), theme.text_secondary()),
                Span::styled(pr_text, theme.text()),
                Span::styled(format!(" ({})", state), state_style),
            ]));
        }
    }

    // Involved entities section
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        " Involved entities",
        theme.subtitle().add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(Span::styled(
        " ─".repeat(18),
        theme.text_muted(),
    )));

    if task.entities.is_empty() {
        lines.push(Line::from(vec![
            Span::styled(format!("  {} ", ENTITY_ICON), theme.warning()),
            Span::styled("No linked entities", theme.warning()),
        ]));
    } else {
        for entity in &task.entities {
            let entity_type = entity.entity_type.as_deref().unwrap_or("unknown");
            let entity_name = entity.name.as_deref().unwrap_or("Unknown");
            let type_icon = match entity_type {
                "stack" => symbols::STACK,
                "environment" => symbols::GEAR,
                "repository" => "📦",
                _ => ENTITY_ICON,
            };
            lines.push(Line::from(vec![
                Span::styled(format!("  {} ", type_icon), theme.accent()),
                Span::styled(format!("{}: ", entity_type), theme.text_muted()),
                Span::styled(entity_name, theme.text()),
            ]));
        }
    }

    // Active policies section
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        " Active policies",
        theme.subtitle().add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(Span::styled(
        " ─".repeat(18),
        theme.text_muted(),
    )));

    if task.policies.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("  ", theme.text_muted()),
            Span::styled("Set up policy groups to enforce", theme.text_muted()),
        ]));
        lines.push(Line::from(vec![
            Span::styled("  ", theme.text_muted()),
            Span::styled("guardrails on infrastructure changes.", theme.text_muted()),
        ]));
    } else {
        for policy in &task.policies {
            let policy_name = policy.name.as_deref().unwrap_or("Unknown");
            let enforcement = policy.enforcement_level.as_deref().unwrap_or("");
            let enforcement_style = match enforcement.to_lowercase().as_str() {
                "mandatory" => theme.error(),
                "advisory" => theme.warning(),
                _ => theme.text_muted(),
            };
            lines.push(Line::from(vec![
                Span::styled(format!("  {} ", POLICY_ICON), theme.text_secondary()),
                Span::styled(policy_name, theme.text()),
                if !enforcement.is_empty() {
                    Span::styled(format!(" ({})", enforcement), enforcement_style)
                } else {
                    Span::raw("")
                },
            ]));
        }
    }

    // Footer hint
    lines.push(Line::from(""));
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        " Press d or Esc to close ",
        theme.text_muted(),
    )));

    let details_para = Paragraph::new(lines).wrap(ratatui::widgets::Wrap { trim: false });
    frame.render_widget(details_para, inner);
}
