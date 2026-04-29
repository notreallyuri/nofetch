#[derive(Debug, PartialEq)]
pub enum StatKind {
    Os,
    Title,
    OsAge,
    Kernel,
    Uptime,
    Memory,
    Cpu,
    Packages,
    Wm,
    Display,
    Gpu,
    GpuDriver,
    Disk,
    Shell,
}

impl StatKind {
    pub fn default_label(&self) -> &'static str {
        match self {
            StatKind::Os => "OS",
            StatKind::Kernel => "Kernel",
            StatKind::Uptime => "Uptime",
            StatKind::Memory => "Memory",
            StatKind::Cpu => "CPU",
            StatKind::Packages => "Packages",
            StatKind::Wm => "WM",
            StatKind::Display => "Display",
            StatKind::Gpu => "GPU",
            StatKind::OsAge => "OS Age",
            StatKind::GpuDriver => "GPU Driver",
            StatKind::Shell => "Shell",
            StatKind::Title | StatKind::Disk => "",
        }
    }
}
