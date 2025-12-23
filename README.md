# Lazy Pulumi

A stylish terminal UI for Pulumi Cloud, ESC, and Neo built with Ratatui.

> [!NOTE]
> **New in this version:** Neo now supports slash commands! Press `/` in the Neo view to access predefined prompts like `/get-started`, `/component-version-report`, and more. Commands are highlighted in purple and can be combined with custom text.

## Features

- **Dashboard**: Overview of your Pulumi resources with quick stats
- **Stacks View**: Browse and manage your Pulumi stacks with update history
- **ESC View**: Manage ESC environments, view definitions, and resolve secrets
- **Neo Chat**: Interactive chat interface for Pulumi's AI agent with markdown rendering
- **Platform View**: Browse Services, Components (Registry Packages), and Templates
- **Organization Selector**: Switch between organizations on-the-fly with `o`
- **Splash Screen**: Startup checks for token validation and CLI availability
- **Log Viewer**: Built-in log viewer for debugging with `l`

## Prerequisites

- Rust 1.82+ (uses latest Ratatui)
- Pulumi Access Token
- Pulumi CLI (checked on startup)

## Setup

Set your Pulumi access token:

```bash
export PULUMI_ACCESS_TOKEN="pul-xxxxxxxxxxxx"

# Optional: Set default organization
export PULUMI_ORG="your-org-name"

# Optional: Custom API endpoint (defaults to https://api.pulumi.com)
export PULUMI_API_URL="https://api.pulumi.com"
```

## Installation

### Homebrew (macOS/Linux)

```bash
brew tap dirien/dirien
brew install lazy-pulumi
```

### From Source

```bash
# Build
cargo build --release

# Run
cargo run --release

# Run with debug logging
RUST_LOG=debug cargo run --release
```

### From Releases

Download the latest binary from the [GitHub Releases](https://github.com/dirien/lazy-pulumi/releases) page.

## Updating

### Homebrew

To update to the latest version via Homebrew:

```bash
brew update && brew upgrade --cask lazy-pulumi
```

Or update all Homebrew packages including lazy-pulumi:

```bash
brew update && brew upgrade
```

> [!NOTE]
> Since `lazy-pulumi` is distributed as a Cask from a third-party tap, running `brew update` first is required to refresh the tap metadata before upgrading.

## Logging

Logs are written to a file to avoid interfering with the TUI:
- **Log file location**: `~/.cache/lazy-pulumi/app.log`
- Press `l` globally to open the log viewer popup
- Logs are color-coded by level (ERROR=red, WARN=yellow, INFO=blue, DEBUG=muted)

## Keyboard Shortcuts

### Global
| Key | Action |
|-----|--------|
| `Tab` / `Shift+Tab` | Switch between views |
| `o` | Open organization selector |
| `l` | Open log viewer |
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

### Neo View
| Key | Action |
|-----|--------|
| `n` | Start new task |
| `i` | Focus input field |
| `/` | Open slash command picker |
| `d` | Show task details (in full-width chat mode) |
| `Enter` | Send message / Load selected task / Insert command |
| `Tab` | Insert selected command (in picker) |
| `Esc` | Show task list (exit full-width chat) / Unfocus input / Close picker |
| `j` / `k` | Scroll chat down/up (3 lines) |
| `J` / `K` | Scroll chat by page |
| `g` | Jump to oldest messages |
| `G` | Jump to newest messages + enable auto-scroll |
| `Page Up/Down` | Scroll messages |
| `↑` / `↓` | Navigate command picker |

### Platform View
| Key | Action |
|-----|--------|
| `h` / `←` | Previous sub-tab |
| `l` / `→` | Next sub-tab |
| `j` / `k` | Navigate list |
| `Enter` | Select item |

### Log Viewer
| Key | Action |
|-----|--------|
| `l` / `Esc` | Close logs |
| `w` | Toggle word wrap |
| `j` / `↓` | Scroll down 3 lines |
| `k` / `↑` | Scroll up 3 lines |
| `J` / `PageDown` | Scroll down by page |
| `K` / `PageUp` | Scroll up by page |
| `g` | Jump to top |
| `G` | Jump to bottom |
| `R` | Refresh logs |

### Splash Screen
| Key | Action |
|-----|--------|
| `Enter` | Continue (when checks pass) |
| `Space` | Toggle "Don't show again" |
| `q` | Quit (when checks fail) |

## Architecture

The application follows **The Elm Architecture (TEA)** pattern for clear separation of concerns.

```
src/
├── main.rs          # Entry point
├── app/             # Application core (TEA pattern)
│   ├── mod.rs       # App struct, new(), run(), render()
│   ├── types.rs     # Model: Tab, FocusMode, AppState, async result types
│   ├── handlers.rs  # Update: All keyboard event handlers
│   ├── data.rs      # Data loading & refresh logic
│   └── neo.rs       # Neo AI agent async operations
├── event.rs         # Event handling (keyboard, mouse)
├── tui.rs           # Terminal setup/teardown
├── theme.rs         # Official Pulumi brand colors & styling
├── config.rs        # User configuration (splash screen preference)
├── startup.rs       # Startup validation checks
├── logging.rs       # File-based logging system
├── api/             # Pulumi API client
│   ├── mod.rs
│   ├── client.rs    # HTTP client
│   └── types.rs     # Data structures (Stacks, ESC, Neo, Resources, Registry)
├── components/      # Reusable UI components
│   ├── mod.rs
│   ├── input.rs     # Text input field
│   ├── list.rs      # Stateful list
│   └── spinner.rs   # Loading spinner
└── ui/              # View rendering
    ├── mod.rs
    ├── dashboard.rs # Overview with stats widgets
    ├── stacks.rs    # Stack list and update history
    ├── esc.rs       # ESC environments with YAML/resolved values
    ├── neo.rs       # Chat interface for Pulumi's AI agent
    ├── platform.rs  # Services, Components, Templates browser
    ├── header.rs    # Tab bar with organization display
    ├── help.rs      # Keyboard shortcut overlay
    ├── logs.rs      # Log viewer popup
    ├── splash.rs    # Startup splash screen with checklist
    └── markdown.rs  # Markdown rendering for Neo messages
```

## Color Theme

The UI uses the official Pulumi brand color palette:

| Color | Hex | Usage |
|-------|-----|-------|
| **Yellow** | #f7bf2a | Accents, highlights, warnings |
| **Salmon** | #f26e7e | Errors, failed states |
| **Fuchsia** | #bd4c85 | Special highlights |
| **Purple** | #8a3391 | Brand accent |
| **Violet** | #805ac3 | Primary accent, focused borders |
| **Blue** | #4d5bd9 | Secondary accent, info states |

Additional UI colors:
- **Success**: Green (#48BB78) for passed states
- **Background**: Dark theme with purple undertones

## Neo Chat Features

The Neo view provides a rich chat interface for Pulumi's AI agent:

- **Slash Commands**: Press `/` to access predefined prompts (e.g., `/get-started`, `/component-version-report`)
  - Commands appear in a picker with descriptions
  - Insert commands with Enter or Tab, then add custom text
  - Multiple commands can be combined in a single message
  - Commands are highlighted with purple background in the input
- **Markdown Rendering**: Bold, italic, code blocks, headers, lists
- **Auto-scroll**: Automatically scrolls to new messages
- **Task Details Dialog**: Press `d` to view task metadata including:
  - Status (idle, running, completed, failed)
  - Started by (user info)
  - Linked PRs with state (open/merged/closed)
  - Involved entities (stacks, environments, repositories)
  - Active policies
- **Thinking Indicator**: Animated spinner while Neo is processing
- **Background Polling**: Updates automatically every few seconds

## Splash Screen

On startup, the application displays a splash screen with:
- Pulumi logo (scaled to terminal size)
- Version information
- Startup checklist:
  - PULUMI_ACCESS_TOKEN validation
  - Pulumi CLI availability check
- Option to skip splash screen on future launches

## License

MIT
