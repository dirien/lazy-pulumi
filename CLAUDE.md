# CLAUDE.md

This file provides guidance to Claude Code when working with this repository.

## Build & Run Commands

```bash
cargo build              # Debug build
cargo build --release    # Release build
cargo run --release      # Run (release recommended)
cargo check              # Check for errors
RUST_LOG=debug cargo run --release  # With debug logging
```

## Environment Variables

```bash
export PULUMI_ACCESS_TOKEN="pul-xxxxxxxxxxxx"  # Required
export PULUMI_ORG="your-org-name"               # Optional
export PULUMI_API_URL="https://api.pulumi.com"  # Optional
```

The token can also be stored in `.env` file (just the token, no variable name).

## Documentation Map

This documentation is distributed across multiple CLAUDE.md files for context efficiency:

| Location | Content |
|----------|---------|
| `src/CLAUDE.md` | Architecture overview, TEA pattern, app flow |
| `src/api/CLAUDE.md` | API endpoints, pagination, request/response types |
| `src/app/CLAUDE.md` | App core, handlers, Neo chat state, polling |
| `src/components/CLAUDE.md` | Reusable widgets (StatefulList, TextInput, TextEditor) |
| `src/ui/CLAUDE.md` | View rendering, dashboard, Neo chat, markdown |

Read the relevant CLAUDE.md when working in that directory.

## Project Structure

```
src/
├── CLAUDE.md       # Architecture overview
├── app/            # Application core (TEA pattern)
│   └── CLAUDE.md   # App state, handlers, Neo polling
├── api/            # Pulumi Cloud API client
│   └── CLAUDE.md   # API endpoints, pagination
├── commands/       # Pulumi CLI command execution
│   ├── mod.rs      # Module exports
│   ├── types.rs    # Command definitions, categories, parameters
│   └── executor.rs # PTY-based command execution
├── components/     # Reusable UI widgets
│   └── CLAUDE.md   # Widget documentation
├── ui/             # View layer (rendering)
│   └── CLAUDE.md   # View rendering details
├── config.rs       # User configuration
├── event.rs        # Async event handler (crossterm)
├── logging.rs      # tui-logger initialization
├── startup.rs      # Startup checks
├── theme.rs        # UI theme/colors
├── tui.rs          # Terminal setup/teardown
└── main.rs         # Entry point
```

## Quick Reference

- **TEA Pattern**: Model (types.rs) → Update (handlers.rs) → View (ui/)
- **State**: `FocusMode::Normal` vs `FocusMode::Input` for navigation vs text input
- **Async**: Uses tokio channels for background operations
- **Logging**: Press `l` globally to open log viewer (tui-logger)
