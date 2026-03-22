<!-- Managed by agent: keep sections and order; edit content, not structure. Last updated: 2026-03-22 -->

# AGENTS.md — src/components/

## Overview
Generic, reusable UI widgets used across multiple views.

**Related docs**: `../ui/AGENTS.md` (view rendering), `../app/AGENTS.md` (state management)

## Key Files

| File | Widget | Purpose |
|------|--------|---------|
| `list.rs` | `StatefulList<T>` | Scrollable list with selection |
| `input.rs` | `TextInput` | Single-line text input with cursor |
| `editor.rs` | `TextEditor` | Multi-line editor with syntax highlighting |
| `spinner.rs` | `Spinner` | Animated loading indicator |

## StatefulList\<T\>

Scrollable list with selection tracking.

```rust
let mut list = StatefulList::with_items(vec![...]);
list.next();     // Select next item
list.previous(); // Select previous item
list.selected(); // Get selected item
```

## TextInput

Single-line input with cursor positioning.

```rust
let mut input = TextInput::new();
input.insert('a');
input.backspace();
input.value();   // Get current text
input.cursor();  // Get cursor position
```

## TextEditor

Multi-line editor for ESC environment YAML editing.

Features: syntax highlighting (syntect), line numbers, vertical scrolling, cursor tracking, auto-indent on Enter.

| Key | Action |
|-----|--------|
| Esc | Save and close |
| Ctrl+C | Cancel without saving |
| Arrow keys | Move cursor |
| Home/End | Line start/end |
| Ctrl+Home/End | Document start/end |
| Tab | Insert 2 spaces |
| Ctrl+U/K | Delete to line start/end |
| Ctrl+A/E | Line start/end (Emacs) |

## Spinner

Animated loading indicator using braille characters.

```rust
let spinner = Spinner::new();
spinner.frame(); // Returns current animation frame
```

Used in: Loading states, Neo "thinking" indicator.

## Setup
- No additional setup beyond root requirements

## Build & tests
- `cargo test` — widget unit tests
- `cargo clippy -- -D warnings`

## Code style
- Widgets must be generic — no app-specific logic
- Follow `StatefulList<T>` API pattern: `new()`, `next()`, `previous()`, `selected()`
- Widgets should not make API calls or hold application state

## Security
- Sanitize text input before rendering
- Handle edge cases (empty lists, very long input)

## Checklist
- [ ] Widget is generic and reusable
- [ ] Unit tests added
- [ ] No app-specific imports

## Examples
> See `list.rs` for the canonical widget pattern.

## When stuck
- Check `list.rs` as the reference implementation
- Check `../ui/AGENTS.md` for how widgets are used in views
- Check root `AGENTS.md` for project conventions

## House Rules
- Keep widgets generic and reusable — no app-specific logic
- Follow `StatefulList<T>` pattern for new widgets
- Widgets should not make API calls or hold application state
