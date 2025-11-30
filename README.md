# Lazy Pulumi

A stylish terminal UI for Pulumi Cloud, ESC, and NEO built with Ratatui.

## Features

- **Dashboard**: Overview of your Pulumi resources with quick stats
- **Stacks View**: Browse and manage your Pulumi stacks with update history
- **ESC View**: Manage ESC environments, view definitions, and resolve secrets
- **NEO Chat**: Interactive chat interface for Pulumi's AI agent
- **Organization Selector**: Switch between organizations on-the-fly with `O`

## Prerequisites

- Rust 1.82+ (uses latest Ratatui)
- Pulumi Access Token

## Setup

Set your Pulumi access token:

```bash
export PULUMI_ACCESS_TOKEN="pul-xxxxxxxxxxxx"

# Optional: Set default organization
export PULUMI_ORG="your-org-name"
```

## Build & Run

```bash
# Build
cargo build --release

# Run
cargo run --release
```

## Keyboard Shortcuts

### Global
| Key | Action |
|-----|--------|
| `Tab` / `Shift+Tab` | Switch between views |
| `o` | Open organization selector |
| `?` | Toggle help |
| `q` / `Ctrl+C` | Quit |
| `r` | Refresh data |
| `Esc` | Close popup / Cancel |

### Navigation
| Key | Action |
|-----|--------|
| `j` / `↓` | Move down |
| `k` / `↑` | Move up |
| `g` / `Home` | First item |
| `G` / `End` | Last item |
| `Enter` | Select / Confirm |

### Stacks View
| Key | Action |
|-----|--------|
| `Enter` | View stack details |
| `u` | View update history |

### ESC View
| Key | Action |
|-----|--------|
| `Enter` | Load environment definition |
| `O` | Open & resolve environment values |

### NEO View
| Key | Action |
|-----|--------|
| `n` | Start new task |
| `i` | Focus input field |
| `Enter` | Send message |
| `Esc` | Unfocus input |
| `Page Up/Down` | Scroll messages |

## Architecture

```
src/
├── main.rs          # Entry point
├── app.rs           # Application state & main loop
├── event.rs         # Event handling (keyboard, mouse)
├── tui.rs           # Terminal setup/teardown
├── theme.rs         # Colors & styling
├── api/             # Pulumi API client
│   ├── mod.rs
│   ├── client.rs    # HTTP client
│   ├── types.rs     # Data structures
│   ├── stacks.rs
│   ├── esc.rs
│   └── neo.rs
├── components/      # Reusable UI components
│   ├── mod.rs
│   ├── input.rs     # Text input field
│   ├── list.rs      # Stateful list
│   └── spinner.rs   # Loading spinner
└── ui/              # View rendering
    ├── mod.rs
    ├── dashboard.rs
    ├── stacks.rs
    ├── esc.rs
    ├── neo.rs
    ├── header.rs
    └── help.rs
```

## Color Theme

The UI uses a Pulumi-inspired color palette:
- **Primary**: Purple (#8A5EFF)
- **Secondary**: Soft Blue (#63B3ED)
- **Accent**: Warm Orange (#F6AD55)
- **Success**: Green (#48BB78)
- **Warning**: Orange (#F6AD55)
- **Error**: Red (#F56565)

## License

MIT
