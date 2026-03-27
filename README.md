[![Powered by Claude Code](https://img.shields.io/badge/Powered%20by-Claude%20Code-orange?style=for-the-badge)](https://claude.ai/code)

# ttop

A minimal TUI process monitor for macOS/Linux, written in Rust. Displays CPU usage, memory usage, and processes grouped by application.

## Features

- **CPU & Memory Gauges** - Real-time system resource monitoring
- **Process Grouping** - Processes grouped by parent application with aggregated stats
- **Collapsible Groups** - Expand/collapse app groups to see child processes
- **Multiple Sort Modes** - Sort by CPU, Memory, Disk I/O, or Name
- **Ascending/Descending Toggle** - Press sort key twice to reverse order
- **Search/Filter** - Filter apps and processes by name

## Installation

```bash
# Clone the repository
git clone https://github.com/yourusername/ttop.git
cd ttop

# Build release version
cargo build --release

# Run
./target/release/ttop
```

## Usage

```bash
cargo run
```

Or after building:

```bash
./target/release/ttop
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
│ ██████████████████░░░░░░░░░░░░░░░░░░░░ 12.4 GB / 32.0 GB │
├─ Apps (156) [Sort: CPU ▼] ────────────────────────┤
│ PID      NAME                        CPU%▼   MEM        DISK   │
│ 1234     ▶ Chrome                    45.2%   2.1 GB     156 MB │
│ 5678     ▼ Terminal                  12.1%   512 MB     32 MB  │
│ 5679        └─ zsh                    8.2%   128 MB     8 MB   │
│ 5680        └─ cargo                  3.9%   384 MB     24 MB  │
│ 9012     ▶ Slack                      5.3%   1.2 GB     64 MB  │
└───────────────────────────────────────────────────┘
q:Quit s:Search ↑↓:Nav x:Toggle c:CPU m:Mem d:Disk n:Name
```

## Dependencies

- [ratatui](https://github.com/ratatui-org/ratatui) - TUI framework
- [crossterm](https://github.com/crossterm-rs/crossterm) - Terminal manipulation
- [sysinfo](https://github.com/GuillaumeGomez/sysinfo) - System information

## License

MIT
