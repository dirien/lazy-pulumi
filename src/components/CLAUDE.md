# src/components/ - Reusable UI Widgets

Generic widgets used across multiple views.

**Related docs**: `../ui/CLAUDE.md` (view rendering), `../app/CLAUDE.md` (state management)

## Files

| File | Widget | Purpose |
|------|--------|---------|
| `list.rs` | `StatefulList<T>` | Scrollable list with selection |
| `input.rs` | `TextInput` | Single-line text input with cursor |
| `editor.rs` | `TextEditor` | Multi-line editor with syntax highlighting |
| `spinner.rs` | `Spinner` | Animated loading indicator |

## StatefulList<T>

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

Features:
- Syntax highlighting (syntect)
- Line numbers in gutter
- Vertical scrolling
- Cursor line/column tracking
- Auto-indent on Enter

Key bindings:
| Key | Action |
|-----|--------|
| `Esc` | Save and close |
| `Ctrl+C` | Cancel without saving |
| Arrow keys | Move cursor |
| `Home/End` | Line start/end |
| `Ctrl+Home/End` | Document start/end |
| `Tab` | Insert 2 spaces |
| `Ctrl+U/K` | Delete to line start/end |
| `Ctrl+A/E` | Line start/end (Emacs) |

## Spinner

Animated loading indicator using braille characters.

```rust
let spinner = Spinner::new();
spinner.frame(); // Returns current animation frame
```

Used in: Loading states, Neo "thinking" indicator
