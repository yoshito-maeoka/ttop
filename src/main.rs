use std::collections::{HashMap, HashSet};
use std::io;
use std::time::Duration;

use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph, Row, Table, TableState},
    Terminal,
};
use sysinfo::{Pid, Process, System};

// macOS-specific: Get the responsible PID for a process
// This is what Activity Monitor uses to group processes by app
#[cfg(target_os = "macos")]
mod macos {
    use std::ffi::c_void;
    use std::mem;
    use std::sync::OnceLock;

    type ResponsibilityFn = unsafe extern "C" fn(i32) -> i32;
    type ProcPidInfoFn = unsafe extern "C" fn(i32, i32, u64, *mut c_void, i32) -> i32;

    // proc_info flavors
    const PROC_PIDT_SHORTBSDINFO: i32 = 13;

    #[repr(C)]
    struct ProcBsdShortInfo {
        pbsi_pid: u32,
        pbsi_ppid: u32,
        pbsi_pgid: u32,
        pbsi_status: u32,
        pbsi_comm: [u8; 16],
        pbsi_flags: u32,
        pbsi_uid: u32,
        pbsi_gid: u32,
        pbsi_ruid: u32,
        pbsi_rgid: u32,
        pbsi_svuid: u32,
        pbsi_svgid: u32,
        pbsi_rfu: u32,
    }

    static RESPONSIBILITY_FN: OnceLock<Option<ResponsibilityFn>> = OnceLock::new();
    static PROC_PIDINFO_FN: OnceLock<Option<ProcPidInfoFn>> = OnceLock::new();

    fn get_responsibility_fn() -> Option<ResponsibilityFn> {
        *RESPONSIBILITY_FN.get_or_init(|| unsafe {
            let handle = libc::dlopen(std::ptr::null(), libc::RTLD_LAZY);
            if handle.is_null() {
                return None;
            }
            let symbol_name = b"responsibility_get_pid_responsible_for_pid\0";
            let symbol = libc::dlsym(handle, symbol_name.as_ptr() as *const i8);
            if symbol.is_null() {
                None
            } else {
                Some(std::mem::transmute::<*mut c_void, ResponsibilityFn>(symbol))
            }
        })
    }

    fn get_proc_pidinfo_fn() -> Option<ProcPidInfoFn> {
        *PROC_PIDINFO_FN.get_or_init(|| unsafe {
            let handle = libc::dlopen(std::ptr::null(), libc::RTLD_LAZY);
            if handle.is_null() {
                return None;
            }
            let symbol_name = b"proc_pidinfo\0";
            let symbol = libc::dlsym(handle, symbol_name.as_ptr() as *const i8);
            if symbol.is_null() {
                None
            } else {
                Some(std::mem::transmute::<*mut c_void, ProcPidInfoFn>(symbol))
            }
        })
    }

    /// Get process group ID - processes in the same app often share this
    pub fn get_process_group(pid: u32) -> Option<u32> {
        let func = get_proc_pidinfo_fn()?;
        unsafe {
            let mut info: ProcBsdShortInfo = mem::zeroed();
            let size = mem::size_of::<ProcBsdShortInfo>() as i32;

            let ret = func(
                pid as i32,
                PROC_PIDT_SHORTBSDINFO,
                0,
                &mut info as *mut _ as *mut c_void,
                size,
            );

            if ret > 0 {
                Some(info.pbsi_pgid)
            } else {
                None
            }
        }
    }

    pub fn get_responsible_pid(pid: u32) -> Option<u32> {
        // Try using responsibility_get_pid_responsible_for_pid if available
        if let Some(func) = get_responsibility_fn() {
            let responsible = unsafe { func(pid as i32) };
            if responsible > 0 && responsible != pid as i32 {
                return Some(responsible as u32);
            }
        }
        None
    }

    /// Check if a process name matches known helper patterns for an app
    /// Returns the app name if this is a known helper process
    pub fn get_parent_app_name(process_name: &str) -> Option<&'static str> {
        let name_lower = process_name.to_lowercase();

        // Safari's WebKit processes
        if name_lower.contains("webkit") && name_lower.starts_with("com.apple.webkit") {
            return Some("Safari");
        }

        // Google Chrome helpers
        if name_lower.contains("google chrome helper") {
            return Some("Google Chrome");
        }

        // Firefox helpers
        if name_lower.contains("firefox") && name_lower.contains("helper") {
            return Some("Firefox");
        }

        // Slack helpers
        if name_lower.contains("slack") && name_lower.contains("helper") {
            return Some("Slack");
        }

        // VS Code helpers
        if name_lower.contains("code helper") || name_lower.contains("electron helper") {
            if name_lower.contains("code") {
                return Some("Code");
            }
        }

        None
    }
}

#[cfg(not(target_os = "macos"))]
mod macos {
    pub fn get_responsible_pid(_pid: u32) -> Option<u32> {
        None // Not supported on non-macOS
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SortMode {
    Cpu,
    Memory,
    DiskIo,
    Name,
}

impl SortMode {
    fn label(&self) -> &'static str {
        match self {
            SortMode::Cpu => "CPU",
            SortMode::Memory => "MEM",
            SortMode::DiskIo => "DISK",
            SortMode::Name => "NAME",
        }
    }
}

#[derive(Clone)]
struct ProcessGroup {
    pid: Pid,
    name: String,
    cpu_usage: f32,
    memory: u64,
    disk_read: u64,
    disk_write: u64,
    children: Vec<ProcessInfo>,
    expanded: bool,
}

#[derive(Clone)]
struct ProcessInfo {
    pid: Pid,
    name: String,
    cpu_usage: f32,
    memory: u64,
    disk_read: u64,
    disk_write: u64,
}

struct App {
    system: System,
    table_state: TableState,
    groups: Vec<ProcessGroup>,
    expanded_pids: HashSet<Pid>,
    sort_mode: SortMode,
    sort_ascending: bool,
    search_active: bool,
    filter_text: String,
}

impl App {
    fn new() -> Self {
        let mut system = System::new_all();
        system.refresh_all();

        let mut table_state = TableState::default();
        table_state.select(Some(0));

        let mut app = Self {
            system,
            table_state,
            groups: Vec::new(),
            expanded_pids: HashSet::new(),
            sort_mode: SortMode::Cpu,
            sort_ascending: false,
            search_active: false,
            filter_text: String::new(),
        };
        app.build_groups();
        app
    }

    fn refresh(&mut self) {
        self.system.refresh_all();
        self.build_groups();
    }

    fn build_groups(&mut self) {
        let processes = self.system.processes();

        // Map each PID to its responsible PID (the app that owns it)
        // On macOS, use the responsible process API
        // On other systems, fall back to parent PID
        let mut responsible_map: HashMap<Pid, Pid> = HashMap::new();

        // First pass: get responsible PIDs using macOS API
        for (pid, _proc) in processes.iter() {
            let pid_u32 = pid.as_u32();

            // Try to get the responsible PID (macOS-specific)
            if let Some(resp_pid) = macos::get_responsible_pid(pid_u32) {
                if resp_pid != pid_u32 && processes.contains_key(&Pid::from_u32(resp_pid)) {
                    responsible_map.insert(*pid, Pid::from_u32(resp_pid));
                }
            }
        }

        // Second pass: for processes without a responsible PID, try heuristics
        // This helps group Safari's WebKit processes and other known app helpers
        #[cfg(target_os = "macos")]
        {
            // Build a map of app names to their PIDs
            let mut app_name_to_pid: HashMap<String, Pid> = HashMap::new();
            for (pid, proc) in processes.iter() {
                let name = proc.name().to_string_lossy().to_string();
                // Store the main app process (not helpers)
                if macos::get_parent_app_name(&name).is_none() {
                    app_name_to_pid.insert(name.to_lowercase(), *pid);
                }
            }

            // Map helper processes to their parent apps
            for (pid, proc) in processes.iter() {
                if responsible_map.contains_key(pid) {
                    continue;
                }
                let name = proc.name().to_string_lossy().to_string();
                if let Some(parent_app) = macos::get_parent_app_name(&name) {
                    // Find the parent app's PID
                    let parent_lower = parent_app.to_lowercase();
                    if let Some(&parent_pid) = app_name_to_pid.get(&parent_lower) {
                        if parent_pid != *pid {
                            responsible_map.insert(*pid, parent_pid);
                        }
                    }
                }
            }

            // Also try process group for remaining processes
            let mut pgid_to_leader: HashMap<u32, Pid> = HashMap::new();

            // Find process group leaders (processes where PID == PGID)
            for (pid, _proc) in processes.iter() {
                if responsible_map.contains_key(pid) {
                    continue;
                }
                if let Some(pgid) = macos::get_process_group(pid.as_u32()) {
                    if pgid == pid.as_u32() {
                        pgid_to_leader.insert(pgid, *pid);
                    }
                }
            }

            // Map non-leader processes to their group leader
            for (pid, _proc) in processes.iter() {
                if responsible_map.contains_key(pid) {
                    continue;
                }
                if let Some(pgid) = macos::get_process_group(pid.as_u32()) {
                    if pgid != pid.as_u32() {
                        if let Some(&leader) = pgid_to_leader.get(&pgid) {
                            if processes.contains_key(&leader) {
                                responsible_map.insert(*pid, leader);
                            }
                        }
                    }
                }
            }
        }

        // Group processes by responsible PID
        let mut children_map: HashMap<Pid, Vec<Pid>> = HashMap::new();
        let mut root_pids: Vec<Pid> = Vec::new();

        for (pid, proc) in processes.iter() {
            // Check if this process has a responsible parent
            if let Some(&resp_pid) = responsible_map.get(pid) {
                children_map.entry(resp_pid).or_default().push(*pid);
            } else {
                // Fall back to parent PID logic
                let parent_pid = proc.parent();

                if let Some(ppid) = parent_pid {
                    // Check if parent has a different responsible process
                    // If so, this process is actually a root
                    let parent_resp = responsible_map.get(&ppid).copied();
                    let my_resp = responsible_map.get(pid).copied();

                    if parent_resp == my_resp && processes.contains_key(&ppid) && ppid.as_u32() > 1 {
                        children_map.entry(ppid).or_default().push(*pid);
                    } else {
                        root_pids.push(*pid);
                    }
                } else {
                    root_pids.push(*pid);
                }
            }
        }

        // Build groups from root processes
        let mut groups: Vec<ProcessGroup> = Vec::new();

        for root_pid in root_pids {
            if let Some(proc) = processes.get(&root_pid) {
                let mut group = ProcessGroup {
                    pid: root_pid,
                    name: proc.name().to_string_lossy().to_string(),
                    cpu_usage: proc.cpu_usage(),
                    memory: proc.memory(),
                    disk_read: proc.disk_usage().read_bytes,
                    disk_write: proc.disk_usage().written_bytes,
                    children: Vec::new(),
                    expanded: self.expanded_pids.contains(&root_pid),
                };

                // Collect all children (processes whose responsible PID is this root)
                self.collect_children(&mut group, root_pid, &children_map, processes);

                // Aggregate stats from children
                for child in &group.children {
                    group.cpu_usage += child.cpu_usage;
                    group.memory += child.memory;
                    group.disk_read += child.disk_read;
                    group.disk_write += child.disk_write;
                }

                groups.push(group);
            }
        }

        // Sort groups
        self.sort_groups(&mut groups);
        self.groups = groups;
    }

    fn collect_children(
        &self,
        group: &mut ProcessGroup,
        parent_pid: Pid,
        children_map: &HashMap<Pid, Vec<Pid>>,
        processes: &HashMap<Pid, Process>,
    ) {
        if let Some(child_pids) = children_map.get(&parent_pid) {
            for child_pid in child_pids {
                if let Some(proc) = processes.get(child_pid) {
                    let child_info = ProcessInfo {
                        pid: *child_pid,
                        name: proc.name().to_string_lossy().to_string(),
                        cpu_usage: proc.cpu_usage(),
                        memory: proc.memory(),
                        disk_read: proc.disk_usage().read_bytes,
                        disk_write: proc.disk_usage().written_bytes,
                    };
                    group.children.push(child_info);

                    // Recursively collect grandchildren
                    self.collect_children(group, *child_pid, children_map, processes);
                }
            }
        }
    }

    fn sort_groups(&self, groups: &mut Vec<ProcessGroup>) {
        let asc = self.sort_ascending;

        match self.sort_mode {
            SortMode::Cpu => {
                groups.sort_by(|a, b| {
                    let cmp = a.cpu_usage.partial_cmp(&b.cpu_usage).unwrap_or(std::cmp::Ordering::Equal);
                    if asc { cmp } else { cmp.reverse() }
                });
            }
            SortMode::Memory => {
                groups.sort_by(|a, b| {
                    let cmp = a.memory.cmp(&b.memory);
                    if asc { cmp } else { cmp.reverse() }
                });
            }
            SortMode::DiskIo => {
                groups.sort_by(|a, b| {
                    let cmp = (a.disk_read + a.disk_write).cmp(&(b.disk_read + b.disk_write));
                    if asc { cmp } else { cmp.reverse() }
                });
            }
            SortMode::Name => {
                groups.sort_by(|a, b| {
                    let cmp = a.name.to_lowercase().cmp(&b.name.to_lowercase());
                    if asc { cmp } else { cmp.reverse() }
                });
            }
        }

        // Sort children within each group
        for group in groups.iter_mut() {
            match self.sort_mode {
                SortMode::Cpu => {
                    group.children.sort_by(|a, b| {
                        let cmp = a.cpu_usage.partial_cmp(&b.cpu_usage).unwrap_or(std::cmp::Ordering::Equal);
                        if asc { cmp } else { cmp.reverse() }
                    });
                }
                SortMode::Memory => {
                    group.children.sort_by(|a, b| {
                        let cmp = a.memory.cmp(&b.memory);
                        if asc { cmp } else { cmp.reverse() }
                    });
                }
                SortMode::DiskIo => {
                    group.children.sort_by(|a, b| {
                        let cmp = (a.disk_read + a.disk_write).cmp(&(b.disk_read + b.disk_write));
                        if asc { cmp } else { cmp.reverse() }
                    });
                }
                SortMode::Name => {
                    group.children.sort_by(|a, b| {
                        let cmp = a.name.to_lowercase().cmp(&b.name.to_lowercase());
                        if asc { cmp } else { cmp.reverse() }
                    });
                }
            }
        }
    }

    fn get_cpu_usage(&self) -> f32 {
        self.system.global_cpu_usage()
    }

    fn get_memory_usage(&self) -> (u64, u64) {
        (self.system.used_memory(), self.system.total_memory())
    }

    fn get_visible_rows(&self) -> Vec<DisplayRow> {
        let mut rows = Vec::new();
        let filter_lower = self.filter_text.to_lowercase();

        for group in &self.groups {
            // Check if group or any child matches filter
            let group_matches = self.filter_text.is_empty()
                || group.name.to_lowercase().contains(&filter_lower);
            let children_match: Vec<&ProcessInfo> = group
                .children
                .iter()
                .filter(|c| {
                    self.filter_text.is_empty()
                        || c.name.to_lowercase().contains(&filter_lower)
                })
                .collect();

            // Show group if it matches or has matching children
            if group_matches || !children_match.is_empty() {
                let has_children = !group.children.is_empty();
                rows.push(DisplayRow {
                    pid: group.pid,
                    name: group.name.clone(),
                    cpu_usage: group.cpu_usage,
                    memory: group.memory,
                    disk_io: group.disk_read + group.disk_write,
                    is_group: true,
                    expanded: group.expanded,
                    has_children,
                });

                if group.expanded {
                    // Show only matching children when filter is active
                    let children_to_show: Vec<&ProcessInfo> = if self.filter_text.is_empty() {
                        group.children.iter().collect()
                    } else {
                        children_match
                    };

                    for child in children_to_show {
                        rows.push(DisplayRow {
                            pid: child.pid,
                            name: child.name.clone(),
                            cpu_usage: child.cpu_usage,
                            memory: child.memory,
                            disk_io: child.disk_read + child.disk_write,
                            is_group: false,
                            expanded: false,
                            has_children: false,
                        });
                    }
                }
            }
        }
        rows
    }

    fn visible_row_count(&self) -> usize {
        self.get_visible_rows().len()
    }

    fn next(&mut self) {
        let count = self.visible_row_count();
        if count == 0 {
            return;
        }
        let i = match self.table_state.selected() {
            Some(i) => {
                if i >= count - 1 {
                    i // Stay at bottom
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.table_state.select(Some(i));
    }

    fn previous(&mut self) {
        let count = self.visible_row_count();
        if count == 0 {
            return;
        }
        let i = match self.table_state.selected() {
            Some(i) => {
                if i == 0 {
                    0 // Stay at top
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.table_state.select(Some(i));
    }

    fn toggle_current(&mut self) {
        let rows = self.get_visible_rows();
        if let Some(selected) = self.table_state.selected() {
            if let Some(row) = rows.get(selected) {
                if row.is_group && row.has_children {
                    if self.expanded_pids.contains(&row.pid) {
                        self.expanded_pids.remove(&row.pid);
                    } else {
                        self.expanded_pids.insert(row.pid);
                    }
                    // Rebuild groups to update expanded state
                    self.build_groups();
                }
            }
        }
    }

    fn toggle_sort_mode(&mut self, mode: SortMode) {
        if self.sort_mode == mode {
            self.sort_ascending = !self.sort_ascending;
        } else {
            self.sort_mode = mode;
            self.sort_ascending = false;
        }
        self.build_groups();
    }
}

struct DisplayRow {
    pid: Pid,
    name: String,
    cpu_usage: f32,
    memory: u64,
    disk_io: u64,
    is_group: bool,
    expanded: bool,
    has_children: bool,
}

fn main() -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();
    let result = run_app(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = result {
        eprintln!("Error: {:?}", err);
    }

    Ok(())
}

fn run_app<B: ratatui::backend::Backend>(terminal: &mut Terminal<B>, app: &mut App) -> Result<()> {
    loop {
        terminal.draw(|f| ui(f, app))?;

        if event::poll(Duration::from_millis(500))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    if app.search_active {
                        // Search input mode
                        match key.code {
                            KeyCode::Esc => {
                                // Reset filter and close search
                                app.filter_text.clear();
                                app.search_active = false;
                                app.table_state.select(Some(0));
                            }
                            KeyCode::Enter => {
                                // Keep filter, close search input
                                app.search_active = false;
                            }
                            KeyCode::Backspace => {
                                app.filter_text.pop();
                                app.table_state.select(Some(0));
                            }
                            KeyCode::Char(c) => {
                                app.filter_text.push(c);
                                app.table_state.select(Some(0));
                            }
                            _ => {}
                        }
                    } else {
                        // Normal mode
                        match key.code {
                            KeyCode::Char('q') => return Ok(()),
                            KeyCode::Esc => {
                                // Clear filter if active, otherwise quit
                                if !app.filter_text.is_empty() {
                                    app.filter_text.clear();
                                    app.table_state.select(Some(0));
                                } else {
                                    return Ok(());
                                }
                            }
                            KeyCode::Down | KeyCode::Char('j') => app.next(),
                            KeyCode::Up | KeyCode::Char('k') => app.previous(),
                            KeyCode::Char(' ') | KeyCode::Char('x') => app.toggle_current(),
                            KeyCode::Char('c') => app.toggle_sort_mode(SortMode::Cpu),
                            KeyCode::Char('m') => app.toggle_sort_mode(SortMode::Memory),
                            KeyCode::Char('d') => app.toggle_sort_mode(SortMode::DiskIo),
                            KeyCode::Char('n') => app.toggle_sort_mode(SortMode::Name),
                            KeyCode::Char('s') => {
                                app.search_active = true;
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        app.refresh();
    }
}

fn ui(f: &mut ratatui::Frame, app: &mut App) {
    let base_constraints = if app.search_active || !app.filter_text.is_empty() {
        vec![
            Constraint::Length(3),  // CPU
            Constraint::Length(3),  // Memory
            Constraint::Length(3),  // Search input
            Constraint::Min(10),    // Process list
            Constraint::Length(1),  // Help
        ]
    } else {
        vec![
            Constraint::Length(3),  // CPU
            Constraint::Length(3),  // Memory
            Constraint::Min(10),    // Process list
            Constraint::Length(1),  // Help
        ]
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints(base_constraints)
        .split(f.area());

    // CPU Gauge
    let cpu_usage = app.get_cpu_usage();
    let cpu_gauge = Gauge::default()
        .block(Block::default().title(" CPU ").borders(Borders::ALL))
        .gauge_style(Style::default().fg(Color::Cyan))
        .percent(cpu_usage.min(100.0) as u16)
        .label(format!("{:.1}%", cpu_usage));
    f.render_widget(cpu_gauge, chunks[0]);

    // Memory Gauge
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
    f.render_widget(mem_gauge, chunks[1]);

    // Determine chunk indices based on whether search bar is shown
    let (table_chunk, help_chunk) = if app.search_active || !app.filter_text.is_empty() {
        // Search input
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
        f.render_widget(search_input, chunks[2]);
        (3, 4)
    } else {
        (2, 3)
    };

    // Process Table
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

    // Show filtered count if filter is active
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

    f.render_stateful_widget(table, chunks[table_chunk], &mut app.table_state);

    // Help line
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
    f.render_widget(help, chunks[help_chunk]);
}

fn format_memory(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}
