# src/app/ - Application Core (TEA Pattern)

Central state machine managing UI state, data, and event loop.

**Related docs**: `../CLAUDE.md` (architecture), `../api/CLAUDE.md` (API calls), `../ui/CLAUDE.md` (rendering), `../components/CLAUDE.md` (widgets)

## Files

| File | Lines | Purpose |
|------|-------|---------|
| `mod.rs` | ~530 | App struct, new(), run(), render() |
| `types.rs` | ~205 | Model: Tab, FocusMode, AppState |
| `handlers.rs` | ~615 | Update: All keyboard event handlers |
| `data.rs` | ~305 | Data loading & refresh logic |
| `neo.rs` | ~270 | Neo AI agent async operations |

## TEA Implementation

- **Model** (`types.rs`): `AppState`, `Tab`, `FocusMode`, `PlatformView`, async result enums
- **Update** (`handlers.rs`): `handle_key()` dispatches to tab-specific handlers
- **View** (`mod.rs`): `render()` method produces UI from state

## Key Types

```rust
enum Tab { Dashboard, Commands, Neo, Stacks, Esc, Platform }
enum FocusMode { Normal, Input }
struct AppState { stacks, environments, neo_tasks, resources, ... }
```

## Event Handlers (handlers.rs)

- `handle_key()` - Main dispatcher
- `handle_stacks_key()` - Stacks tab navigation
- `handle_esc_key()` - ESC environments
- `handle_neo_key()` - Neo chat (see below)
- `handle_platform_key()` - Platform view
- `handle_commands_key()` - Commands tab (see below)

## Neo Chat State Variables

| Variable | Type | Purpose |
|----------|------|---------|
| `neo_polling` | bool | Active polling after sending message |
| `neo_poll_counter` | u8 | Ticks since last poll |
| `neo_stable_polls` | u8 | Consecutive polls with no new messages |
| `neo_bg_poll_counter` | u8 | Background poll counter |
| `neo_scroll_state` | ScrollViewState | Scroll position (tui-scrollview) |
| `neo_auto_scroll` | Arc<AtomicBool> | Thread-safe auto-scroll toggle |
| `neo_task_is_running` | bool | Task status is "running" |
| `neo_show_command_picker` | bool | Show slash command picker popup |
| `neo_filtered_commands` | Vec<NeoSlashCommand> | Filtered commands for picker |
| `neo_command_picker_index` | usize | Selected command in picker |

## Neo Polling Mechanism

- **Active polling**: Every 500ms (5 ticks) after sending
- **Background polling**: Every 3s (30 ticks) when Neo tab active
- **Stop conditions**: Task not running + has assistant response, or timeout

## Neo Key Bindings

| Key | Action |
|-----|--------|
| `i` | Enter input mode |
| `/` | Open slash command picker |
| `n` | New task |
| `d` | Task details dialog |
| `j/k` | Scroll 3 lines |
| `J/K` | Page scroll |
| `g/G` | Jump to top/bottom |
| `Enter` | Load selected task |
| `Esc` | Show task list |

## Neo Slash Commands

Slash commands allow users to invoke predefined Neo prompts.

### How it works
1. Commands are fetched from `GET /api/console/agents/{org}/commands`
2. When user types `/` in input, picker shows filtered commands
3. User navigates with ↑/↓, selects with Enter or Tab to complete
4. Sends to `POST /api/preview/agents/{org}/tasks` with command payload:
   ```json
   {
     "message": {
       "type": "user_message",
       "content": "{{cmd:name:tag}}",
       "timestamp": "...",
       "commands": { "{{cmd:name:tag}}": { ... command details ... } }
     }
   }
   ```

### Input Mode Key Bindings (when picker showing)
| Key | Action |
|-----|--------|
| `↑/↓` or `Ctrl+P/N` | Navigate commands |
| `Tab` | Insert command into input |
| `Enter` | Insert command into input |
| `Esc` | Cancel picker |

Note: Commands are inserted (not immediately executed) so users can add text or multiple commands before sending.

## Data Loading (data.rs)

Uses tokio channels for parallel async requests. Sets `is_loading` flag during requests.

## Startup Checks

`spawn_startup_checks()` spawns background tasks for:
- `PULUMI_ACCESS_TOKEN` validation
- Pulumi CLI availability (`pulumi version`)

Uses `StartupCheckResult` enum and tokio channel.

## Commands Tab

Executes Pulumi CLI commands with streaming output via PTY.

### View States (`CommandsViewState`)
- `BrowsingCategories` - Navigating command categories
- `BrowsingCommands` - Navigating commands in selected category
- `InputDialog` - Filling command parameters
- `ConfirmDialog` - Confirming destructive commands
- `OutputView` - Viewing command output

### Key Bindings
| Key | Context | Action |
|-----|---------|--------|
| `↑/↓` | Categories/Commands | Navigate |
| `→/Enter` | Categories | Enter commands list |
| `←` | Commands | Back to categories |
| `Enter` | Commands | Run selected command |
| `Tab` | InputDialog | Next parameter |
| `y/n` | ConfirmDialog | Confirm/cancel |
| `j/k` | OutputView | Scroll 3 lines |
| `g/G` | OutputView | Top/bottom |
| `Esc` | OutputView | Close |

### PTY Execution
Commands run in pseudo-TTY via `portable-pty` crate for proper streaming output.
Deduplication filters repeated progress lines from Pulumi output.
