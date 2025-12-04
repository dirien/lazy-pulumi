# src/ui/ - View Layer

Renders application state to Ratatui frames.

**Related docs**: `../app/CLAUDE.md` (state & handlers), `../components/CLAUDE.md` (widgets), `../api/CLAUDE.md` (data types)

## Files

| File | View | Description |
|------|------|-------------|
| `dashboard.rs` | Dashboard | Stats cards, resource chart, recent updates |
| `commands.rs` | Commands | Pulumi CLI command execution with streaming output |
| `stacks.rs` | Stacks | Stack list and update history |
| `esc.rs` | ESC | Environments list, YAML editor |
| `neo.rs` | Neo | Chat interface with markdown rendering |
| `platform.rs` | Platform | Services, Components, Templates |
| `header.rs` | Header | Tab bar with organization display |
| `help.rs` | Help | Keyboard shortcut overlay |
| `logs.rs` | Logs | tui-logger widget |
| `splash.rs` | Splash | Startup loading screen |
| `markdown.rs` | - | Markdown parsing for Neo messages |
| `syntax.rs` | - | Syntax highlighting (syntect) |

## Dashboard Features

1. **Stats Cards** (top row): Stacks, Environments, Tasks, Resources
   - Uses `tui-big-text` with `PixelSize::Quadrant`

2. **Resource Chart** (full-width): Line chart over 30 days
   - `Chart` widget with `GraphType::Line`, `Marker::Braille`

3. **Recent Updates** (bottom left): Last 5 unique stack updates
4. **Quick Info** (bottom right): Keyboard shortcuts

## ESC Editor

`render_esc_editor()` renders the YAML editor dialog:
- Line numbers in gutter
- Syntax highlighting via syntect
- Vertical scrollbar
- `[modified]` indicator in title

## Neo Chat Rendering

Uses `tui-scrollview` for proper scroll handling.

### Markdown Support
- **Bold**: `**text**` or `__text__`
- *Italic*: `*text*` or `_text_`
- `Inline code`: backticks
- Code blocks: triple backticks with language labels
- Headers: `#`, `##`, `###`
- Lists: `-`, `*`, `1.`, `2.`

### Thinking Indicator
Dedicated 2-line area between chat and input:
- Visible when: `neo_polling || is_loading || neo_task_is_running`
- Animated spinner with "Neo is thinking..."

### Message Types
- `UserMessage` - User input
- `AssistantMessage` - Neo response (markdown rendered)
- `ToolCall` - Tool execution notification
- `ToolResponse` - Tool result (truncated)
- `ApprovalRequest` - Requires user approval
- `TaskNameChange` - Task renamed

## Ratatui LLM Chat Best Practices

1. **Use tui-scrollview**: `Paragraph::scroll()` doesn't handle wrapped lines
2. **Auto-scroll**: `Arc<AtomicBool>` for thread-safe toggle
3. **Thinking indicator**: Dedicated layout area, not inline
4. **Background polling**: Poll every few seconds when tab active

Reference: [Tenere](https://github.com/pythops/tenere), [tui-scrollview](https://github.com/joshka/tui-scrollview)

## Commands View

Renders Pulumi CLI command execution interface (LazyGit-style).

### Layout
- **Left panel**: Command categories and commands list
- **Input dialog**: Parameter input fields (popup)
- **Confirm dialog**: Yes/No for destructive commands
- **Output view**: Streaming command output with scroll

### Output Colorization
`colorize_pulumi_output()` applies colors based on content:
- Green: created, succeeded, unchanged
- Red: deleted, failed, error
- Yellow: updated, warning
- Cyan: reading, refreshing
