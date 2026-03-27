# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with this repository.

## Project Overview

**ttop** is a terminal-based (TUI) process monitor written in Rust that displays CPU usage, memory usage, and processes grouped by application. It's similar to `htop` but with an app-centric tree view like macOS Activity Monitor.

## Build Commands

```bash
# Build debug version
cargo build

# Build release version
cargo build --release

# Run directly
cargo run

# Install to ~/.cargo/bin
cargo install --path .

# Run the installed binary
ttop
```

## Architecture

### Main Components

- **`src/main.rs`** - Single-file application containing:
  - `App` struct - Main application state (system info, UI state, filters, sort mode)
  - `ProcessGroup` / `ProcessInfo` - Data structures for grouped processes
  - `macos` module - macOS-specific FFI for responsible process lookup
  - `run_app()` - Main event loop handling keyboard input
  - `ui()` - Renders the TUI using ratatui widgets

### Process Grouping Strategy (macOS)

The app uses multiple strategies to group processes by their parent application:

1. **Responsible Process API** - Uses `responsibility_get_pid_responsible_for_pid()` dynamically loaded from libSystem
2. **App Name Heuristics** - Maps known helper processes to parent apps:
   - `com.apple.WebKit.*` → Safari
   - `Google Chrome Helper*` → Google Chrome
   - `Firefox*Helper` → Firefox
   - `Slack*Helper` → Slack
   - `Code Helper*` → VS Code
3. **Process Group ID** - Falls back to PGID-based grouping
4. **Parent PID** - Final fallback using traditional parent-child relationships

### Key Dependencies

- `ratatui` - TUI rendering framework
- `crossterm` - Cross-platform terminal handling
- `sysinfo` - System/process information
- `libc` (macOS only) - For `dlopen`/`dlsym` to load macOS APIs

## Adding New App Heuristics

To add grouping support for a new app's helper processes, edit the `get_parent_app_name()` function in the `macos` module:

```rust
pub fn get_parent_app_name(process_name: &str) -> Option<&'static str> {
    let name_lower = process_name.to_lowercase();

    // Add your pattern here
    if name_lower.contains("myapp") && name_lower.contains("helper") {
        return Some("MyApp");
    }

    // ... existing patterns
}
```

## UI Layout

```
┌─ CPU ─────────────────────────────┐  <- Gauge widget
├─ Memory ──────────────────────────┤  <- Gauge widget
├─ Search ──────────────────────────┤  <- (Optional) Paragraph widget
├─ Apps (N) [Sort: X] ──────────────┤  <- Table widget with TableState
│ PID    NAME           CPU  MEM    │
│ ...                               │
└───────────────────────────────────┘
q:Quit s:Search ...                    <- Help line (Paragraph)
```

## Testing

Currently no automated tests. Manual testing:

1. Run `cargo run`
2. Verify CPU/Memory gauges update
3. Test navigation with `j`/`k` or arrow keys
4. Test expand/collapse with `x` or `Space`
5. Test search with `s`, type filter, `Enter` to apply, `Esc` to clear
6. Test sort modes with `c`, `m`, `d`, `n`
7. Verify Safari's WebKit processes group under Safari (macOS)
