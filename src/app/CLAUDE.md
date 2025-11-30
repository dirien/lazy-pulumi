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
enum Tab { Dashboard, Stacks, Esc, Neo, Platform }
enum FocusMode { Normal, Input }
struct AppState { stacks, environments, neo_tasks, resources, ... }
```

## Event Handlers (handlers.rs)

- `handle_key()` - Main dispatcher
- `handle_stacks_key()` - Stacks tab navigation
- `handle_esc_key()` - ESC environments
- `handle_neo_key()` - Neo chat (see below)
- `handle_platform_key()` - Platform view

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

## Neo Polling Mechanism

- **Active polling**: Every 500ms (5 ticks) after sending
- **Background polling**: Every 3s (30 ticks) when Neo tab active
- **Stop conditions**: Task not running + has assistant response, or timeout

## Neo Key Bindings

| Key | Action |
|-----|--------|
| `i` | Enter input mode |
| `n` | New task |
| `d` | Task details dialog |
| `j/k` | Scroll 3 lines |
| `J/K` | Page scroll |
| `g/G` | Jump to top/bottom |
| `Enter` | Load selected task |
| `Esc` | Show task list |

## Data Loading (data.rs)

Uses tokio channels for parallel async requests. Sets `is_loading` flag during requests.

## Startup Checks

`spawn_startup_checks()` spawns background tasks for:
- `PULUMI_ACCESS_TOKEN` validation
- Pulumi CLI availability (`pulumi version`)

Uses `StartupCheckResult` enum and tokio channel.
