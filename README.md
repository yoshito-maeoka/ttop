[![Powered by Claude Code](https://img.shields.io/badge/Powered%20by-Claude%20Code-orange?style=for-the-badge)](https://claude.ai/code)

# ttop

A minimal TUI process monitor for macOS/Linux, written in Rust. Displays CPU usage, memory usage, and processes grouped by application - similar to Activity Monitor's tree view.

## Features

- **CPU & Memory Gauges** - Real-time system resource monitoring with visual bars
- **App-based Process Grouping** - Processes grouped by their parent application with aggregated stats
  - On macOS: Uses the "responsible process" API (same as Activity Monitor)
  - Includes heuristics for WebKit, Chrome helpers, and other known app patterns
- **Collapsible Groups** - Expand/collapse app groups to see child processes
- **Multiple Sort Modes** - Sort by CPU, Memory, Disk I/O, or Name
- **Ascending/Descending Toggle** - Press sort key twice to reverse order
- **Search/Filter** - Filter apps and processes by name in real-time

## Installation

### From source

```bash
# Clone the repository
git clone https://github.com/yoshito-maeoka/ttop.git
cd ttop

# Build release version
cargo build --release

# Install to ~/.cargo/bin
cargo install --path .
```

### Run directly

```bash
cargo run
```

## Keybindings

| Key | Action |
|-----|--------|
| `q` | Quit |
| `Esc` | Clear filter / Quit |
| `↑` / `k` | Move cursor up |
| `↓` / `j` | Move cursor down |
| `x` / `Space` | Toggle expand/collapse group |
| `s` | Activate search input |
| `Enter` | Apply filter (in search mode) |
| `Backspace` | Delete character (in search mode) |
| `c` | Sort by CPU usage |
| `m` | Sort by Memory usage |
| `d` | Sort by Disk I/O |
| `n` | Sort by Name |

**Note:** Press the same sort key twice to toggle between ascending (▲) and descending (▼) order.

## Screenshot

```
┌─ CPU ─────────────────────────────────────────────┐
│ ████████████░░░░░░░░░░░░░░░░░░░░░░░░░░ 32.5%      │
├─ Memory ──────────────────────────────────────────┤
│ ██████████████████░░░░░░░░░░░░░░░░░░░░ 12.4/32.0 GB │
├─ Apps (156) [Sort: CPU ▼] ────────────────────────┤
│ PID      NAME                         CPU%▼  MEM       DISK    │
│ 1234     ▼ Safari                     25.2%  1.8 GB    256 MB  │
│ 2345        └─ com.apple.WebKit...     12.1%  512 MB    64 MB   │
│ 2346        └─ com.apple.WebKit...      8.3%  384 MB    32 MB   │
│ 5678     ▶ Google Chrome              18.5%  2.1 GB    156 MB  │
│ 9012     ▶ Terminal                    5.3%  256 MB    32 MB   │
└───────────────────────────────────────────────────┘
q:Quit s:Search ↑↓:Nav x:Toggle c:CPU m:Mem d:Disk n:Name
```

## Platform Support

| Platform | Process Grouping | Notes |
|----------|-----------------|-------|
| macOS | Full support | Uses responsible process API + heuristics |
| Linux | Basic support | Groups by parent PID |

## Dependencies

- [ratatui](https://github.com/ratatui-org/ratatui) - TUI framework
- [crossterm](https://github.com/crossterm-rs/crossterm) - Terminal manipulation
- [sysinfo](https://github.com/GuillaumeGomez/sysinfo) - System information
- [libc](https://github.com/rust-lang/libc) - macOS system calls (macOS only)

## License

MIT
