<!-- Managed by agent: keep sections and order; edit content, not structure. Last updated: 2026-03-22 -->

# AGENTS.md — src/

## Overview
Terminal UI application for Pulumi Cloud built with Ratatui and Tokio. Uses The Elm Architecture (TEA) pattern.

**Related docs**: See root `AGENTS.md` for build commands. Subdirectory docs: `app/AGENTS.md`, `api/AGENTS.md`, `components/AGENTS.md`, `ui/AGENTS.md`

## The Elm Architecture (TEA) Pattern

| Layer | Location | Responsibility |
|-------|----------|----------------|
| Model | `app/types.rs` | Pure data types defining application state |
| Update | `app/handlers/` | Event handlers that modify state (split by tab) |
| View | `app/mod.rs` + `ui/` | Renders state to terminal |

## Application Flow

1. `main.rs` initializes color-eyre, tui-logger, creates `App`, calls `app.run()`
2. `App::new()` sets up terminal, event handler, API client, loads initial data
3. `App::run()` enters async loop: render -> poll events -> handle input
4. `handlers/` dispatches to tab-specific handler modules
5. API calls are async and set `is_loading` flag during requests

## State Management

- `FocusMode::Normal` vs `FocusMode::Input` controls navigation vs text input
- Popup states (`show_help`, `show_org_selector`, `error`) overlay main content
- Each view has a `StatefulList` for selection tracking

## Key Files

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
- `spawn_startup_checks()` in `handlers/startup.rs` spawns background tasks
- Uses `StartupCheckResult` enum and tokio channel
- Checks: `PULUMI_ACCESS_TOKEN` env var, Pulumi CLI availability

## Logging

Press `l` globally to open log viewer. Key bindings:
- `h`: Toggle target selector
- `f`: Focus on selected target
- Up/Down: Select target
- Left/Right: Change shown log level
- PageUp/PageDown: Scroll history

## Setup
- Rust stable toolchain via `rustup`
- `PULUMI_ACCESS_TOKEN` env var required (see root `AGENTS.md`)

## Build & tests
- `cargo check` — fast error check
- `cargo test` — run all tests
- `cargo clippy -- -D warnings` — lint

## Code style
- `cargo fmt` before committing
- `Result<T, E>` for recoverable errors, `.expect("reason")` for invariants
- `&str` over `String` when possible; iterators over collecting

## Security
- Never log or display `PULUMI_ACCESS_TOKEN`
- Validate all API response data before use

## Checklist
- [ ] `cargo test` passes
- [ ] `cargo clippy -- -D warnings` clean
- [ ] `cargo fmt --check` clean

## Examples
> See **Golden Samples** in root `AGENTS.md` for canonical patterns.

## When stuck
- Check root `AGENTS.md` for project conventions
- Read scoped `AGENTS.md` in the directory you're editing
