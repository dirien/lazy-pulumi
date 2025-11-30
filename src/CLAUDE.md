# src/ - Architecture Overview

Terminal UI application for Pulumi Cloud built with Ratatui and Tokio.

**Related docs**: See `../CLAUDE.md` for build commands. Subdirectory docs: `app/CLAUDE.md`, `api/CLAUDE.md`, `components/CLAUDE.md`, `ui/CLAUDE.md`

## The Elm Architecture (TEA) Pattern

1. **Model** (`app/types.rs`): Pure data types defining application state
2. **Update** (`app/handlers.rs`): Event handlers that modify state
3. **View** (`app/mod.rs` + `ui/`): Renders state to terminal

## Application Flow

1. `main.rs` initializes color-eyre, tui-logger, creates `App`, calls `app.run()`
2. `App::new()` sets up terminal, event handler, API client, loads initial data
3. `App::run()` enters async loop: render → poll events → handle input
4. `handlers.rs` dispatches to tab-specific handlers
5. API calls are async and set `is_loading` flag during requests

## State Management

- `FocusMode::Normal` vs `FocusMode::Input` controls navigation vs text input
- Popup states (`show_help`, `show_org_selector`, `error`) overlay main content
- Each view has a `StatefulList` for selection tracking

## Key Modules

| File | Purpose |
|------|---------|
| `main.rs` | Entry point, initializes app |
| `tui.rs` | Terminal setup/teardown (crossterm backend) |
| `event.rs` | Async event handler, tick events for animations |
| `config.rs` | User configuration |
| `theme.rs` | UI colors and styles |
| `logging.rs` | tui-logger initialization |
| `startup.rs` | Startup validation checks |

## Startup Checks (Async)

Startup checks run asynchronously to keep UI responsive:
- `spawn_startup_checks()` in `handlers.rs` spawns background tasks
- Uses `StartupCheckResult` enum and tokio channel
- Checks: `PULUMI_ACCESS_TOKEN` env var, Pulumi CLI availability

## Logging

Press `l` globally to open log viewer. Key bindings:
- `h`: Toggle target selector
- `f`: Focus on selected target
- `↑/↓`: Select target
- `←/→`: Change shown log level
- `PageUp/PageDown`: Scroll history
