use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph, Row, Table},
    Frame,
};

use crate::app::App;
use crate::domain::SortMode;
use crate::utils::{format_bytes, format_memory};

pub fn render(f: &mut Frame, app: &mut App) {
    let base_constraints = if app.search_active || !app.filter_text.is_empty() {
        vec![
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(1),
        ]
    } else {
        vec![
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(1),
        ]
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints(base_constraints)
        .split(f.area());

    render_cpu_gauge(f, app, chunks[0]);
    render_memory_gauge(f, app, chunks[1]);

    let (table_chunk, help_chunk) = if app.search_active || !app.filter_text.is_empty() {
        render_search_input(f, app, chunks[2]);
        (3, 4)
    } else {
        (2, 3)
    };

    render_process_table(f, app, chunks[table_chunk]);
    render_help_line(f, app, chunks[help_chunk]);
}

fn render_cpu_gauge(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let cpu_usage = app.get_cpu_usage();
    let cpu_gauge = Gauge::default()
        .block(Block::default().title(" CPU ").borders(Borders::ALL))
        .gauge_style(Style::default().fg(Color::Cyan))
        .percent(cpu_usage.min(100.0) as u16)
        .label(format!("{:.1}%", cpu_usage));
    f.render_widget(cpu_gauge, area);
}

fn render_memory_gauge(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let (used_mem, total_mem) = app.get_memory_usage();
    let mem_percent = if total_mem > 0 {
        (used_mem as f64 / total_mem as f64 * 100.0) as u16
    } else {
        0
    };
    let mem_gauge = Gauge::default()
        .block(Block::default().title(" Memory ").borders(Borders::ALL))
        .gauge_style(Style::default().fg(Color::Magenta))
        .percent(mem_percent.min(100))
        .label(format!(
            "{:.1} GB / {:.1} GB ({:.1}%)",
            used_mem as f64 / 1024.0 / 1024.0 / 1024.0,
            total_mem as f64 / 1024.0 / 1024.0 / 1024.0,
            mem_percent
        ));
    f.render_widget(mem_gauge, area);
}

fn render_search_input(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let search_title = if app.search_active {
        " Search (Enter: apply, Esc: clear) "
    } else {
        " Filter (s: edit, Esc: clear) "
    };
    let search_style = if app.search_active {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::Green)
    };
    let search_input = Paragraph::new(format!(" {}", app.filter_text))
        .style(search_style)
        .block(Block::default().title(search_title).borders(Borders::ALL));
    f.render_widget(search_input, area);
}

fn render_process_table(f: &mut Frame, app: &mut App, area: ratatui::layout::Rect) {
    let visible_rows = app.get_visible_rows();
    let rows: Vec<Row> = visible_rows
        .iter()
        .map(|row| {
            let prefix = if row.is_group {
                if row.has_children {
                    if row.expanded { "▼ " } else { "▶ " }
                } else {
                    "  "
                }
            } else {
                "    └─ "
            };

            let name_style = if row.is_group {
                Style::default().add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::DarkGray)
            };

            Row::new(vec![
                format!("{}", row.pid),
                format!("{}{}", prefix, row.name),
                format!("{:.1}%", row.cpu_usage),
                format_memory(row.memory),
                format_bytes(row.disk_io),
            ])
            .style(name_style)
        })
        .collect();

    let header_style = Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD);
    let sort_indicator = if app.sort_ascending { "▲" } else { "▼" };
    let header = Row::new(vec![
        "PID".to_string(),
        if app.sort_mode == SortMode::Name {
            format!("NAME{}", sort_indicator)
        } else {
            "NAME".to_string()
        },
        if app.sort_mode == SortMode::Cpu {
            format!("CPU%{}", sort_indicator)
        } else {
            "CPU%".to_string()
        },
        if app.sort_mode == SortMode::Memory {
            format!("MEM{}", sort_indicator)
        } else {
            "MEM".to_string()
        },
        if app.sort_mode == SortMode::DiskIo {
            format!("DISK{}", sort_indicator)
        } else {
            "DISK".to_string()
        },
    ])
    .style(header_style);

    let title = if app.filter_text.is_empty() {
        format!(
            " Apps ({}) [Sort: {} {}] ",
            app.groups.len(),
            app.sort_mode.label(),
            if app.sort_ascending { "▲" } else { "▼" }
        )
    } else {
        format!(
            " Apps ({} shown) [Sort: {} {}] ",
            visible_rows.iter().filter(|r| r.is_group).count(),
            app.sort_mode.label(),
            if app.sort_ascending { "▲" } else { "▼" }
        )
    };

    let table = Table::new(
        rows,
        [
            Constraint::Length(8),
            Constraint::Min(30),
            Constraint::Length(8),
            Constraint::Length(12),
            Constraint::Length(12),
        ],
    )
    .header(header)
    .block(Block::default().title(title).borders(Borders::ALL))
    .row_highlight_style(Style::default().bg(Color::DarkGray));

    f.render_stateful_widget(table, area, &mut app.table_state);
}

fn render_help_line(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let help = if app.search_active {
        Paragraph::new(Line::from(vec![
            Span::raw("Type to search, "),
            Span::styled("Enter", Style::default().fg(Color::Yellow)),
            Span::raw(": apply filter, "),
            Span::styled("Esc", Style::default().fg(Color::Yellow)),
            Span::raw(": clear & close"),
        ]))
    } else {
        Paragraph::new(Line::from(vec![
            Span::styled("q", Style::default().fg(Color::Yellow)),
            Span::raw(":Quit "),
            Span::styled("s", Style::default().fg(Color::Yellow)),
            Span::raw(":Search "),
            Span::styled("↑↓", Style::default().fg(Color::Yellow)),
            Span::raw(":Nav "),
            Span::styled("x", Style::default().fg(Color::Yellow)),
            Span::raw(":Toggle "),
            Span::styled("c", Style::default().fg(Color::Yellow)),
            Span::raw(":CPU "),
            Span::styled("m", Style::default().fg(Color::Yellow)),
            Span::raw(":Mem "),
            Span::styled("d", Style::default().fg(Color::Yellow)),
            Span::raw(":Disk "),
            Span::styled("n", Style::default().fg(Color::Yellow)),
            Span::raw(":Name"),
        ]))
    };
    f.render_widget(help, area);
}
