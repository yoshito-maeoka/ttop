use std::collections::{HashMap, HashSet};

use ratatui::widgets::TableState;
use sysinfo::{Pid, Process, System};

// macOS-specific: Get the responsible PID for a process
#[cfg(target_os = "macos")]
pub mod macos {
    use std::ffi::c_void;
    use std::mem;
    use std::sync::OnceLock;

    type ResponsibilityFn = unsafe extern "C" fn(i32) -> i32;
    type ProcPidInfoFn = unsafe extern "C" fn(i32, i32, u64, *mut c_void, i32) -> i32;

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
        if let Some(func) = get_responsibility_fn() {
            let responsible = unsafe { func(pid as i32) };
            if responsible > 0 && responsible != pid as i32 {
                return Some(responsible as u32);
            }
        }
        None
    }

    pub fn get_parent_app_name(process_name: &str) -> Option<&'static str> {
        let name_lower = process_name.to_lowercase();

        if name_lower.contains("webkit") && name_lower.starts_with("com.apple.webkit") {
            return Some("Safari");
        }

        if name_lower.contains("google chrome helper") {
            return Some("Google Chrome");
        }

        if name_lower.contains("firefox") && name_lower.contains("helper") {
            return Some("Firefox");
        }

        if name_lower.contains("slack") && name_lower.contains("helper") {
            return Some("Slack");
        }

        if name_lower.contains("code helper") || name_lower.contains("electron helper") {
            if name_lower.contains("code") {
                return Some("Code");
            }
        }

        None
    }
}

#[cfg(not(target_os = "macos"))]
pub mod macos {
    pub fn get_responsible_pid(_pid: u32) -> Option<u32> {
        None
    }

    pub fn get_process_group(_pid: u32) -> Option<u32> {
        None
    }

    pub fn get_parent_app_name(_process_name: &str) -> Option<&'static str> {
        None
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SortMode {
    Cpu,
    Memory,
    DiskIo,
    Name,
}

impl SortMode {
    pub fn label(&self) -> &'static str {
        match self {
            SortMode::Cpu => "CPU",
            SortMode::Memory => "MEM",
            SortMode::DiskIo => "DISK",
            SortMode::Name => "NAME",
        }
    }
}

#[derive(Clone)]
pub struct ProcessGroup {
    pub pid: Pid,
    pub name: String,
    pub cpu_usage: f32,
    pub memory: u64,
    pub disk_read: u64,
    pub disk_write: u64,
    pub children: Vec<ProcessInfo>,
    pub expanded: bool,
}

#[derive(Clone)]
pub struct ProcessInfo {
    pub pid: Pid,
    pub name: String,
    pub cpu_usage: f32,
    pub memory: u64,
    pub disk_read: u64,
    pub disk_write: u64,
}

pub struct DisplayRow {
    pub pid: Pid,
    pub name: String,
    pub cpu_usage: f32,
    pub memory: u64,
    pub disk_io: u64,
    pub is_group: bool,
    pub expanded: bool,
    pub has_children: bool,
}

pub struct App {
    pub system: System,
    pub table_state: TableState,
    pub groups: Vec<ProcessGroup>,
    pub expanded_pids: HashSet<Pid>,
    pub sort_mode: SortMode,
    pub sort_ascending: bool,
    pub search_active: bool,
    pub filter_text: String,
}

impl App {
    pub fn new() -> Self {
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

    pub fn refresh(&mut self) {
        self.system.refresh_all();
        self.build_groups();
    }

    pub fn build_groups(&mut self) {
        let processes = self.system.processes();

        let mut responsible_map: HashMap<Pid, Pid> = HashMap::new();

        for (pid, _proc) in processes.iter() {
            let pid_u32 = pid.as_u32();

            if let Some(resp_pid) = macos::get_responsible_pid(pid_u32) {
                if resp_pid != pid_u32 && processes.contains_key(&Pid::from_u32(resp_pid)) {
                    responsible_map.insert(*pid, Pid::from_u32(resp_pid));
                }
            }
        }

        #[cfg(target_os = "macos")]
        {
            let mut app_name_to_pid: HashMap<String, Pid> = HashMap::new();
            for (pid, proc) in processes.iter() {
                let name = proc.name().to_string_lossy().to_string();
                if macos::get_parent_app_name(&name).is_none() {
                    app_name_to_pid.insert(name.to_lowercase(), *pid);
                }
            }

            for (pid, proc) in processes.iter() {
                if responsible_map.contains_key(pid) {
                    continue;
                }
                let name = proc.name().to_string_lossy().to_string();
                if let Some(parent_app) = macos::get_parent_app_name(&name) {
                    let parent_lower = parent_app.to_lowercase();
                    if let Some(&parent_pid) = app_name_to_pid.get(&parent_lower) {
                        if parent_pid != *pid {
                            responsible_map.insert(*pid, parent_pid);
                        }
                    }
                }
            }

            let mut pgid_to_leader: HashMap<u32, Pid> = HashMap::new();

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

        let mut children_map: HashMap<Pid, Vec<Pid>> = HashMap::new();
        let mut root_pids: Vec<Pid> = Vec::new();

        for (pid, proc) in processes.iter() {
            if let Some(&resp_pid) = responsible_map.get(pid) {
                children_map.entry(resp_pid).or_default().push(*pid);
            } else {
                let parent_pid = proc.parent();

                if let Some(ppid) = parent_pid {
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

                self.collect_children(&mut group, root_pid, &children_map, processes);

                for child in &group.children {
                    group.cpu_usage += child.cpu_usage;
                    group.memory += child.memory;
                    group.disk_read += child.disk_read;
                    group.disk_write += child.disk_write;
                }

                groups.push(group);
            }
        }

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

                    self.collect_children(group, *child_pid, children_map, processes);
                }
            }
        }
    }

    pub fn sort_groups(&self, groups: &mut Vec<ProcessGroup>) {
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

    pub fn get_cpu_usage(&self) -> f32 {
        self.system.global_cpu_usage()
    }

    pub fn get_memory_usage(&self) -> (u64, u64) {
        (self.system.used_memory(), self.system.total_memory())
    }

    pub fn get_visible_rows(&self) -> Vec<DisplayRow> {
        let mut rows = Vec::new();
        let filter_lower = self.filter_text.to_lowercase();

        for group in &self.groups {
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

    pub fn visible_row_count(&self) -> usize {
        self.get_visible_rows().len()
    }

    pub fn next(&mut self) {
        let count = self.visible_row_count();
        if count == 0 {
            return;
        }
        let i = match self.table_state.selected() {
            Some(i) => {
                if i >= count - 1 {
                    i
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.table_state.select(Some(i));
    }

    pub fn previous(&mut self) {
        let count = self.visible_row_count();
        if count == 0 {
            return;
        }
        let i = match self.table_state.selected() {
            Some(i) => {
                if i == 0 {
                    0
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.table_state.select(Some(i));
    }

    pub fn toggle_current(&mut self) {
        let rows = self.get_visible_rows();
        if let Some(selected) = self.table_state.selected() {
            if let Some(row) = rows.get(selected) {
                if row.is_group && row.has_children {
                    if self.expanded_pids.contains(&row.pid) {
                        self.expanded_pids.remove(&row.pid);
                    } else {
                        self.expanded_pids.insert(row.pid);
                    }
                    self.build_groups();
                }
            }
        }
    }

    pub fn toggle_sort_mode(&mut self, mode: SortMode) {
        if self.sort_mode == mode {
            self.sort_ascending = !self.sort_ascending;
        } else {
            self.sort_mode = mode;
            self.sort_ascending = false;
        }
        self.build_groups();
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

pub fn format_memory(bytes: u64) -> String {
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

pub fn format_bytes(bytes: u64) -> String {
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
