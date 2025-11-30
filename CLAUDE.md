# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Run Commands

```bash
# Build (debug)
cargo build

# Build (release with optimizations)
cargo build --release

# Run (release mode recommended for performance)
cargo run --release

# Check for compilation errors without building
cargo check

# Run with debug logging
RUST_LOG=debug cargo run --release
```

## Logging

Logs are written to a file instead of stdout to avoid interfering with the TUI:
- **Log file location**: `~/.cache/lazy-pulumi/app.log`
- Press `l` globally to open the log viewer popup
- Logs are color-coded by level (ERROR=red, WARN=yellow, INFO=blue, DEBUG=muted)

### Log Viewer Key Bindings
| Key | Action |
|-----|--------|
| `l` or `Esc` | Close logs |
| `w` | Toggle word wrap on/off |
| `j` / `↓` | Scroll down 3 lines |
| `k` / `↑` | Scroll up 3 lines |
| `J` / `PageDown` | Scroll down by page |
| `K` / `PageUp` | Scroll up by page |
| `g` | Jump to top |
| `G` | Jump to bottom |
| `R` | Refresh logs |

### Implementation
- `src/logging.rs` - File-based logging initialization and log reading
- `src/ui/logs.rs` - Log viewer popup rendering with word wrap support
- Logs are cached when viewer opens; press `R` to reload from file

## Required Environment Variables

```bash
# Required: Pulumi API authentication
export PULUMI_ACCESS_TOKEN="pul-xxxxxxxxxxxx"

# Optional: Default organization
export PULUMI_ORG="your-org-name"

# Optional: Custom API endpoint (defaults to https://api.pulumi.com)
export PULUMI_API_URL="https://api.pulumi.com"
```

## Credentials

The Pulumi access token is stored in `.env` file (just the token, no variable name):
```
pul-xxxxxxxxxxxx
```

To test API calls manually:
```bash
# Get token from .env
TOKEN=$(cat .env | head -1)

# List Neo tasks for an org
curl -s -H "Content-Type: application/json" \
  -H "Authorization: token $TOKEN" \
  "https://api.pulumi.com/api/preview/agents/{ORG}/tasks"

# Get task events
curl -s -H "Content-Type: application/json" \
  -H "Authorization: token $TOKEN" \
  "https://api.pulumi.com/api/preview/agents/{ORG}/tasks/{TASK_ID}/events"
```

## Neo API (Preview Agents)

The Neo AI agent uses the Preview Agents API:
- **List tasks**: `GET /api/preview/agents/{org}/tasks`
- **Get task metadata**: `GET /api/preview/agents/{org}/tasks/{taskId}` - Returns single task details
- **Create task**: `POST /api/preview/agents/{org}/tasks` with `{"message": {"type": "user_message", "content": "...", "timestamp": "..."}}`
- **Get events**: `GET /api/preview/agents/{org}/tasks/{taskId}/events`
- **Respond**: `POST /api/preview/agents/{org}/tasks/{taskId}` with `{"event": {"type": "user_message", "content": "...", "timestamp": "..."}}`

Event body types in responses:
- `user_message` - User input (has `content`)
- `assistant_message` - Assistant response (has `content`)
- `set_task_name` - Task name change
- `exec_tool_call` - Tool execution (has `tool_calls`)
- `tool_response` - Tool result (has `content` with result)
- `user_approval_request` - Approval request

## Architecture Overview

This is a terminal UI (TUI) application for Pulumi Cloud built with Ratatui and Tokio. The architecture follows **The Elm Architecture (TEA)** pattern for clear separation of concerns.

### Project Structure

```
src/
├── app/                    # Application core (TEA pattern)
│   ├── mod.rs              # App struct, new(), run(), render() (~530 lines)
│   ├── types.rs            # Model: Tab, FocusMode, PlatformView, AppState (~205 lines)
│   ├── handlers.rs         # Update: All keyboard event handlers (~615 lines)
│   ├── data.rs             # Data loading & refresh logic (~305 lines)
│   └── neo.rs              # Neo AI agent async operations (~270 lines)
├── api/                    # Pulumi Cloud API client
│   ├── mod.rs              # Re-exports
│   ├── client.rs           # HTTP client implementation
│   └── types.rs            # API response types
├── components/             # Reusable UI widgets
│   ├── list.rs             # StatefulList<T>
│   ├── input.rs            # TextInput
│   └── spinner.rs          # Loading spinner
├── ui/                     # View layer (rendering)
│   ├── dashboard.rs        # Dashboard view
│   ├── stacks.rs           # Stacks view
│   ├── esc.rs              # ESC environments view
│   ├── neo.rs              # Neo chat view
│   ├── platform.rs         # Platform view
│   └── ...                 # Other UI components
├── config.rs               # User configuration
├── event.rs                # Async event handler
├── logging.rs              # File-based logging
├── startup.rs              # Startup checks
├── theme.rs                # UI theme/colors
├── tui.rs                  # Terminal setup/teardown
└── main.rs                 # Entry point
```

### The Elm Architecture (TEA) Pattern

The application follows TEA principles:

1. **Model** (`app/types.rs`): Pure data types defining application state
   - `AppState` - All fetched data (stacks, environments, tasks, etc.)
   - `Tab`, `FocusMode`, `PlatformView` - UI state enums
   - `DataLoadResult`, `NeoAsyncResult`, `StartupCheckResult` - Async operation results

2. **Update** (`app/handlers.rs`): Event handlers that modify state
   - `handle_key()` - Main keyboard event dispatcher
   - `handle_stacks_key()`, `handle_esc_key()`, `handle_neo_key()`, etc.
   - Pure functions that take current state and produce new state

3. **View** (`app/mod.rs` + `ui/`): Renders state to terminal
   - `render()` method produces UI from current state
   - `ui/` module contains view-specific rendering functions

### Core Components

- **App** (`src/app/mod.rs`): Central state machine managing UI state, data, and the main event loop. Contains `AppState` for data.

- **Handlers** (`src/app/handlers.rs`): All keyboard event handling, organized by context (global, tab-specific, popup-specific).

- **Data** (`src/app/data.rs`): Async data loading with parallel requests using tokio channels.

- **Neo** (`src/app/neo.rs`): Neo AI agent operations including polling, message sending, and task management.

- **API Client** (`src/api/client.rs`): Async HTTP client for Pulumi Cloud REST API. Handles authentication via bearer token and provides methods for Stacks, ESC, Neo, and Resource Search APIs.

- **Event System** (`src/event.rs`): Async event handler using crossterm. Generates tick events for animations and captures keyboard/mouse input.

- **TUI** (`src/tui.rs`): Terminal setup/teardown with crossterm backend. Handles raw mode and alternate screen.

### UI Layer

Views in `src/ui/` render to Ratatui frames:
- `dashboard.rs` - Overview with stats widgets
- `stacks.rs` - Stack list and update history
- `esc.rs` - ESC environments with YAML/resolved values
- `neo.rs` - Chat interface for Pulumi's AI agent
- `platform.rs` - Services, Components, Templates views
- `header.rs` - Tab bar with organization display
- `help.rs` - Keyboard shortcut overlay

### Reusable Components (`src/components/`)

- `StatefulList<T>` - Scrollable list with selection state
- `TextInput` - Single-line text input with cursor
- `Spinner` - Animated loading indicator

### Application Flow

1. `main.rs` initializes color-eyre, tracing, creates `App`, and calls `app.run()`
2. `App::new()` sets up terminal, event handler, API client, loads initial data
3. `App::run()` enters async loop: render frame → poll events → handle input
4. `handlers.rs` dispatches to tab-specific handlers (`handle_stacks_key`, `handle_esc_key`, `handle_neo_key`)
5. API calls are async and set `is_loading` flag during requests

### State Management

- `FocusMode::Normal` vs `FocusMode::Input` controls whether keys go to navigation or text input
- Popup states (`show_help`, `show_org_selector`, `error`) overlay the main content
- Each view has a `StatefulList` for selection tracking

### Startup Checks (Async)

Startup checks run asynchronously to keep the UI responsive:
- **Implementation**: `spawn_startup_checks()` in `handlers.rs` spawns background tasks
- **Communication**: Uses `StartupCheckResult` enum and tokio channel (`startup_result_tx/rx`)
- **Processing**: `process_startup_results()` receives results non-blocking in main loop
- **Benefits**: Spinner animates during CLI version check instead of UI freezing
- **Checks performed**:
  - `PULUMI_ACCESS_TOKEN` environment variable (synchronous but wrapped in task)
  - Pulumi CLI availability via `pulumi version` (async)

## Neo Chat Implementation

### Polling Mechanism
The Neo chat uses async polling to fetch agent responses:
- **Active polling** (after sending message): Every 500ms (5 ticks at 100ms tick rate)
- **Background polling** (when viewing Neo tab): Every 3 seconds (30 ticks)
- **Immediate poll**: Triggered right after task creation
- **Task status aware**: Polls fetch both events AND task status in parallel
- **Stop conditions** for active polling:
  - Task status is NOT "running"/"in_progress"/"pending" AND has assistant response
  - OR max 60 polls (~30 seconds timeout)
  - OR 20+ stable polls AND task is not running (fallback)
- **Thinking indicator**: Stays visible as long as task status is "running"

### Key State Variables (in `App`)
- `neo_polling: bool` - Whether actively polling for responses (fast polling after sending)
- `neo_poll_counter: u8` - Ticks since last poll
- `neo_stable_polls: u8` - Consecutive polls with no new messages
- `neo_bg_poll_counter: u8` - Background poll counter when Neo tab is active
- `neo_scroll_state: ScrollViewState` - Scroll state from tui-scrollview crate
- `neo_auto_scroll: Arc<AtomicBool>` - Thread-safe auto-scroll toggle
- `neo_task_is_running: bool` - Tracks if current task status is "running" (from API)

### Scrolling Implementation (using tui-scrollview)
Uses `tui-scrollview` crate for proper scroll handling (similar to Tenere LLM TUI):
- `ScrollViewState` manages scroll position with proper `scroll_to_bottom()` method
- `Arc<AtomicBool>` for thread-safe auto-scroll toggle (pattern from Tenere)
- Auto-scroll enabled by default, disabled when user scrolls up manually
- Re-enabled when user presses `G` or new messages arrive with auto-scroll on

Key methods used:
- `scroll_state.scroll_up()` / `scroll_down()` - Single line movement
- `scroll_state.scroll_page_up()` / `scroll_page_down()` - Page movement
- `scroll_state.scroll_to_top()` / `scroll_to_bottom()` - Jump to edges
- `scroll_state.offset()` - Get current scroll position for scrollbar

### Thinking Indicator
- Dedicated 2-line area shown between chat and input
- Visible when: `neo_polling || is_loading || neo_task_is_running`
- `neo_task_is_running` ensures banner stays visible until API confirms task is no longer running
- Displays animated spinner with "Neo is thinking..." message
- Centered with background highlight for visibility

### Markdown Rendering
Assistant messages support markdown rendering:
- **Bold** (`**text**` or `__text__`)
- *Italic* (`*text*` or `_text_`)
- `Inline code` (backticks)
- Code blocks with language labels (triple backticks)
- Headers (`#`, `##`, `###`)
- Bullet lists (`-` or `*`)
- Numbered lists (`1.`, `2.`, etc.)

### Neo Tab Key Bindings
| Key | Action |
|-----|--------|
| `i` | Enter input mode to type message |
| `n` | Start new task/conversation |
| `d` | Show task details dialog (only in full-width chat mode) |
| `↑`/`↓` | Navigate task list (left panel) |
| `j` | Scroll chat down 3 lines (newer) |
| `k` | Scroll chat up 3 lines (older) + disable auto-scroll |
| `J`/`PageDown` | Scroll chat down by page |
| `K`/`PageUp` | Scroll chat up by page + disable auto-scroll |
| `g` | Jump to top (oldest messages) + disable auto-scroll |
| `G` | Jump to bottom + re-enable auto-scroll |
| `Enter` | Load selected task's messages |
| `Esc` | Show task list (exit full-width chat mode) |

### Task Details Dialog
Press `d` in full-width chat mode to show task details (similar to Pulumi Cloud web UI):
- **Status**: Task state (idle, running, completed, failed)
- **Started on**: Task creation timestamp
- **Started by**: User who initiated the task
- **Linked PRs**: Associated pull requests with state (open/merged/closed)
- **Involved entities**: Stacks, environments, repositories linked to task
- **Active policies**: Policy groups enforcing guardrails

The dialog fetches fresh data from `GET /api/preview/agents/{org}/tasks/{taskId}` each time it opens.

### Task Data Types
```rust
NeoTask {
    id, name, status, created_at, updated_at, url,
    started_by: Option<NeoTaskUser>,
    linked_prs: Vec<NeoLinkedPR>,
    entities: Vec<NeoEntity>,
    policies: Vec<NeoPolicy>,
}
```

Note: API may return `null` for array fields. Use custom deserializer `null_to_empty_vec` to handle this.

### Message Types (`NeoMessageType`)
- `UserMessage` - User input
- `AssistantMessage` - Neo's response (may include tool_calls, rendered with markdown)
- `ToolCall` - Tool execution notification
- `ToolResponse` - Tool result (truncated display)
- `ApprovalRequest` - Requires user approval
- `TaskNameChange` - Task renamed by agent

## API Pagination Notes

All list APIs require pagination to get accurate counts. Key details:

### ESC Environments API
- **Endpoint**: `GET /api/esc/environments/{org}`
- **Pagination**: Uses `continuationToken` query parameter
- **Response fields**: `environments` array, no `organization` field in each item (implied from URL)
- **Field names**: Uses `created` and `modified` (NOT `createdAt`/`modifiedAt`)
- **Extra fields**: API returns additional fields like `id`, `tags`, `links`, `referrerMetadata`, `settings` - use `#[serde(default)]` to ignore

### Neo Tasks API
- **Endpoint**: `GET /api/preview/agents/{org}/tasks`
- **Pagination**: Uses `pageSize` (default 100, max 1000) and `continuationToken`
- **Response**: `{ tasks: [...], continuationToken: "..." }`

### Resource Search API
- **Endpoint**: `GET /api/orgs/{org}/search/resourcesv2` (note: v2 endpoint)
- **Pagination**: Uses `page` (1-based) and `size` parameters
- **Response**: Includes `pagination.next` URL when more results available
- **Note**: Old endpoint `/api/orgs/{org}/search/resources` is deprecated

### Stacks API
- **Endpoint**: `GET /api/user/stacks?organization={org}`
- **Pagination**: Uses `continuationToken`
- **Response**: `{ stacks: [...], continuationToken: "..." }`

### Recent Stack Updates API (Console)
- **Endpoint**: `GET /api/console/orgs/{org}/stacks/updates/recent?limit=N`
- **Returns**: Array of stacks with their `lastUpdate` containing `requestedBy`, `info`, `version`
- **Used for**: Dashboard "Recent Stack Updates" panel

### Resource Summary API
- **Endpoint**: `GET /api/orgs/{org}/resources/summary?granularity=daily&lookbackDays=N`
- **Returns**: `{ summary: [{ year, month, day, resources, resourceHours }, ...] }`
- **Used for**: Dashboard "Resource Count Over Time" chart

## Dashboard Features

The dashboard displays:

1. **Stats Cards** (top row):
   - Stacks count
   - Environments count
   - Neo Tasks count
   - Resources count

2. **Resource Count Over Time** (full-width chart):
   - Uses ratatui `Chart` widget with `GraphType::Line` and `Marker::Braille`
   - Shows resource count over the last 30 days
   - X-axis: date labels (first and last date)
   - Y-axis: resource count with auto-calculated bounds
   - Data from `/api/orgs/{org}/resources/summary` API

3. **Recent Stack Updates** (bottom left):
   - Shows last 5 unique stack updates (deduplicated by project/stack)
   - Format: `project / stack / Update #N` + `username updated X ago`
   - Data from `/api/console/orgs/{org}/stacks/updates/recent` API

4. **Quick Info** (bottom right):
   - Keyboard shortcuts: Tab (views), ? (help), r (refresh)

## Ratatui LLM Chat Best Practices

When building LLM chat interfaces with Ratatui:

1. **Use tui-scrollview** for scrolling: Manual scroll calculation with `Paragraph::scroll()` doesn't handle wrapped lines correctly. The `tui-scrollview` crate provides proper `ScrollViewState` with `scroll_to_bottom()`.

2. **Auto-scroll pattern**: Use `Arc<AtomicBool>` for thread-safe auto-scroll toggle (pattern from Tenere). Enable on send/receive, disable on manual scroll up, re-enable on scroll to bottom.

3. **Thinking indicator**: Use a dedicated layout area (not inline with messages) for loading/thinking state. This ensures visibility regardless of scroll position.

4. **Background polling**: When tab is active, poll periodically (every few seconds) to catch updates without requiring manual refresh.

5. **Reference implementations**:
   - [Tenere](https://github.com/pythops/tenere) - LLM TUI with auto-scroll, streaming
   - [Oatmeal](https://github.com/dustinblackman/oatmeal) - LLM chat with multiple backends
   - [tui-scrollview](https://github.com/joshka/tui-scrollview) - ScrollView widget for Ratatui
