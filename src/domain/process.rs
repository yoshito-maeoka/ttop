use sysinfo::Pid;

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
