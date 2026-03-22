<!-- Managed by agent: keep sections and order; edit content, not structure. Last updated: 2026-03-22 -->

# AGENTS.md — src/ui/

## Overview
Renders application state to Ratatui frames. Pure view layer — no state mutation.

**Related docs**: `../app/AGENTS.md` (state & handlers), `../components/AGENTS.md` (widgets), `../api/AGENTS.md` (data types)

## Key Files

| File | View | Description |
|------|------|-------------|
| `dashboard.rs` | Dashboard | Stats cards, resource chart, recent updates |
| `commands.rs` | Commands | Pulumi CLI command execution with streaming output |
| `stacks.rs` | Stacks | Stack list and update history |
| `esc.rs` | ESC | Environments list, YAML editor |
| `neo.rs` | Neo | Chat interface with markdown rendering |
| `platform.rs` | Platform | Services, Private Components, Registry, Templates |
| `header.rs` | Header | Tab bar with organization display |
| `help.rs` | Help | Keyboard shortcut overlay |
| `logs.rs` | Logs | tui-logger widget |
| `splash.rs` | Splash | Startup loading screen |
| `markdown.rs` | — | Markdown parsing for Neo messages |
| `syntax.rs` | — | Syntax highlighting (syntect) |

## Dashboard Features

1. **Stats Cards** (top row): Stacks, Environments, Tasks, Resources — uses `tui-big-text` with `PixelSize::Quadrant`
2. **Resource Chart** (full-width): Line chart over 30 days — `Chart` widget with `GraphType::Line`, `Marker::Braille`
3. **Recent Updates** (bottom left): Last 5 unique stack updates
4. **Quick Info** (bottom right): Keyboard shortcuts

## ESC Editor

`render_esc_editor()` renders YAML editor dialog: line numbers in gutter, syntax highlighting via syntect, vertical scrollbar, `[modified]` indicator in title.

## Neo Chat Rendering

Uses `tui-scrollview` for proper scroll handling.

### Markdown Support
Bold (`**text**`), italic (`*text*`), inline code (backticks), code blocks (triple backticks with language labels), headers (`#`/`##`/`###`), lists (`-`/`*`/`1.`).

### Task Settings Bar
Shown above the input field when composing a new task (no `current_task_id`). Displays current approval, permission, and plan mode with key hints (`a`/`p`/`m` to cycle). Uses descriptive labels like "Approval Manual (org default) (a)".

### Plan Mode Visual Indicator
When plan mode is active (either selected for new task or persisted on current task via `plan_mode` field), the chat and input blocks use `BorderType::LightDoubleDashed` with warning color, and the title shows " Chat [PLAN] ".

### Thinking Indicator
Dedicated 2-line area between chat and input. Visible when: `neo_polling || is_loading || neo_task_is_running`. Animated spinner with "Neo is thinking..."

### Message Types
`UserMessage`, `AssistantMessage` (markdown rendered), `ToolCall`, `ToolResponse` (truncated), `ApprovalRequest`, `TaskNameChange`

### Task Details Dialog
Press `d` to show task metadata. Includes plan/execute mode indicator.

### Slash Commands Management Dialog
`render_slash_commands_dialog()` — List view, Detail view (scrollable), Create/Edit forms, Delete confirmation.

## Commands View

LazyGit-style Pulumi CLI execution interface.

### Layout
- **Left panel**: Command categories and commands list
- **Input dialog**: Parameter input fields (popup)
- **Confirm dialog**: Yes/No for destructive commands
- **Output view**: Streaming command output with scroll

### Output Colorization
`colorize_pulumi_output()` applies colors: green (created/succeeded/unchanged), red (deleted/failed/error), yellow (updated/warning), cyan (reading/refreshing).

## Ratatui Best Practices
1. Use `tui-scrollview` — `Paragraph::scroll()` doesn't handle wrapped lines
2. `Arc<AtomicBool>` for thread-safe auto-scroll toggle
3. Dedicated layout area for thinking indicator, not inline
4. Background polling every few seconds when tab active

## Setup
- No additional setup beyond root requirements

## Code style
- Render functions are pure — read state, produce frame, no mutation
- `#[allow(clippy::too_many_arguments)]` is acceptable for render functions
- Use `theme.rs` colors, don't hardcode
- Layout: use `Layout::default().constraints([...])` for splitting

## Security
- Sanitize any user-generated content before rendering
- Truncate long strings to prevent layout overflow

## Checklist
- [ ] Render function is pure (no state mutation)
- [ ] Uses `theme.rs` colors
- [ ] Handles loading/error/empty states
- [ ] `cargo clippy -- -D warnings` clean

## Examples
> See `stacks.rs` for canonical view pattern, `neo.rs` for complex scrollable view.

## When stuck
- Check `../app/AGENTS.md` for state management
- Check `../components/AGENTS.md` for available widgets
- Check root `AGENTS.md` for project conventions

## House Rules
- Render functions are pure — read state, produce frame, no mutation
- `#[allow(clippy::too_many_arguments)]` is acceptable for render functions
- Use `theme.rs` colors, don't hardcode
