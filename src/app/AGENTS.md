<!-- Managed by agent: keep sections and order; edit content, not structure. Last updated: 2026-03-22 -->

# AGENTS.md — src/app/

## Overview
Central state machine managing UI state, data, and event loop. Implements TEA pattern.

**Related docs**: `../AGENTS.md` (architecture), `../api/AGENTS.md` (API calls), `../ui/AGENTS.md` (rendering), `../components/AGENTS.md` (widgets)

## Key Files

| File | ~Lines | Purpose |
|------|--------|---------|
| `mod.rs` | ~880 | App struct, new(), run(), render() |
| `types.rs` | ~470 | Model: Tab, FocusMode, NeoTaskSettings, AppState |
| `handlers.rs` | ~650 | Update: All keyboard event handlers |
| `data.rs` | ~420 | Data loading & refresh logic |
| `neo.rs` | ~440 | Neo AI agent async operations |

## TEA Implementation

| Layer | File | Responsibility |
|-------|------|----------------|
| Model | `types.rs` | `AppState`, `Tab`, `FocusMode`, `PlatformView`, async result enums |
| Update | `handlers.rs` | `handle_key()` dispatches to tab-specific handlers |
| View | `mod.rs` | `render()` method produces UI from state |

## Key Types
```rust
enum Tab { Dashboard, Commands, Neo, Stacks, Esc, Platform }
enum FocusMode { Normal, Input }
enum PlatformView { Services, PrivateComponents, Registry, Templates }
enum NeoApprovalMode { Default, Manual, Auto, Balanced }
enum NeoPermissionMode { Default, Full, ReadOnly }
enum NeoPlanMode { Default, On, Off }
struct NeoTaskSettings { approval_mode, permission_mode, plan_mode }
struct AppState { stacks, environments, neo_tasks, resources, ... }
```

`NeoApprovalMode`, `NeoPermissionMode`, `NeoPlanMode` each have `cycle()`, `label()`, and `api_value()` methods. The `Default` variant shows the actual value (e.g., "Manual (org default)") and returns `None` for `api_value()` so the field is omitted from the API request.

## Event Handlers (handlers.rs)

- `handle_key()` — Main dispatcher
- `handle_stacks_key()` — Stacks tab navigation
- `handle_esc_key()` — ESC environments
- `handle_neo_key()` — Neo chat
- `handle_platform_key()` — Platform view
- `handle_commands_key()` — Commands tab

## Neo Chat State Variables

| Variable | Type | Purpose |
|----------|------|---------|
| `neo_polling` | `bool` | Active polling after sending message |
| `neo_poll_counter` | `u8` | Ticks since last poll |
| `neo_stable_polls` | `u8` | Consecutive polls with no new messages |
| `neo_bg_poll_counter` | `u8` | Background poll counter |
| `neo_scroll_state` | `ScrollViewState` | Scroll position (tui-scrollview) |
| `neo_auto_scroll` | `Arc<AtomicBool>` | Thread-safe auto-scroll toggle |
| `neo_task_is_running` | `bool` | Task status is "running" |
| `neo_show_command_picker` | `bool` | Show slash command picker popup |
| `neo_filtered_commands` | `Vec<NeoSlashCommand>` | Filtered commands for picker |
| `neo_command_picker_index` | `usize` | Selected command in picker |
| `neo_task_settings` | `NeoTaskSettings` | Settings for new task creation (approval/permission/plan) |
| `neo_hide_task_list` | `bool` | Hide task list for full-width chat |

## Neo Polling Mechanism

- **Active polling**: Every 500ms (5 ticks) after sending
- **Background polling**: Every 3s (30 ticks) when Neo tab active
- **Stop conditions**: Task not running + has assistant response, or timeout

## Neo Key Bindings

| Key | Action |
|-----|--------|
| `i` | Enter input mode |
| `/` | Open slash command picker |
| `c` | Open slash commands management dialog |
| `n` | New task |
| `d` | Task details dialog |
| `j/k` | Scroll 3 lines |
| `J/K` | Page scroll |
| `g/G` | Jump to top/bottom |
| `a` | Cycle approval mode (new task only) |
| `p` | Cycle permission mode (new task only) |
| `m` | Cycle plan mode (new task only) |
| `Enter` | Load selected task |
| `Esc` | Show task list |

## Slash Commands Management Dialog

Press `c` in Neo tab. Supports CRUD for custom slash commands.

### Dialog Views (`SlashCommandsDialogView`)
`List` -> `Detail` -> `Create` / `Edit` / `ConfirmDelete`

### Dialog Key Bindings

| View | Key | Action |
|------|-----|--------|
| List | Up/Down | Navigate |
| List | Enter | View detail |
| List | `n` | Create new |
| List | `e` | Edit (custom only) |
| List | `d` | Delete (custom only) |
| List | Esc | Close |
| Detail | `j/k` | Scroll |
| Detail | `e` | Edit |
| Detail | Esc | Back to list |
| Create/Edit | Tab | Next field |
| Create/Edit | Shift+Tab | Previous field |
| Create/Edit | Ctrl+S | Save |
| Create/Edit | Esc | Cancel |

## Data Loading (data.rs)

Uses tokio channels for parallel async requests. Sets `is_loading` flag during requests.

## Commands Tab

Executes Pulumi CLI commands with streaming output via PTY.

### View States (`CommandsViewState`)
`BrowsingCategories` -> `BrowsingCommands` -> `InputDialog` -> `ConfirmDialog` -> `OutputView`

### Commands Key Bindings

| Key | Context | Action |
|-----|---------|--------|
| Up/Down | Categories/Commands | Navigate |
| Right/Enter | Categories | Enter commands list |
| Left | Commands | Back to categories |
| Enter | Commands | Run selected command |
| Tab | InputDialog | Next parameter |
| `y/n` | ConfirmDialog | Confirm/cancel |
| `j/k` | OutputView | Scroll 3 lines |
| `g/G` | OutputView | Top/bottom |
| Esc | OutputView | Close |

PTY execution via `portable-pty` crate with deduplication filtering for repeated progress lines.

## Setup
- No additional setup beyond root requirements

## Code style
- Handlers: `handle_*_key()` naming pattern, always check `FocusMode`
- State: add new fields to `AppState` in `types.rs`, handle in appropriate handler
- Async: use tokio channels, set `is_loading` during requests

## Security
- Validate user input before sending to API
- Sanitize CLI command parameters before PTY execution

## Checklist
- [ ] New `Tab` variant added to `types.rs` if adding a view
- [ ] Handler added to `handlers.rs` with `FocusMode` checks
- [ ] Data loading in `data.rs` if new API data needed
- [ ] `cargo test` passes

## Examples
> See `handlers.rs` for handler patterns, `data.rs` for async loading patterns.

## When stuck
- Check `../AGENTS.md` for architecture overview
- Check `../api/AGENTS.md` for API details
- Check `../ui/AGENTS.md` for rendering patterns
