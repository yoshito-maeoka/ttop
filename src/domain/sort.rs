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
