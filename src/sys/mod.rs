use std::time::SystemTime;
use sysinfo::{Disks, System};

mod displays;
mod gpu;
mod packages;

use crate::sys::{displays::detect_displays, gpu::get_gpu_info, packages::detect_packages};

pub struct SysData {
    pub user: String,
    pub host: String,
    pub os: String,
    pub os_age: String,
    pub kernel: String,
    pub uptime: String,

    pub mem_used_b: u64,
    pub mem_total_b: u64,
    pub disk_used_b: u64,
    pub disk_total_b: u64,

    pub cpu: String,
    pub packages: String,
    pub wm: String,
    pub displays: Vec<String>,
    pub gpu: String,
    pub gpu_driver: String,
}

pub fn gather_info() -> SysData {
    let mut sys = System::new_all();
    sys.refresh_all();

    let live_mhz = sys.cpus().first().map(|c| c.frequency()).unwrap_or(0);
    let live_ghz = live_mhz as f64 / 1000.0;

    let cpu = sys
        .cpus()
        .first()
        .map(|c| {
            let brand = c
                .brand()
                .trim()
                .replace(" 6-Core Processor", "")
                .replace(" 12-Thread Processor", "");

            format!("{} @ {:.2} GHz", brand, live_ghz)
        })
        .unwrap_or_else(|| "Unknown CPU".into());

    let os_age = std::fs::metadata("/")
        .and_then(|m| m.created())
        .map(|created| {
            let duration = SystemTime::now()
                .duration_since(created)
                .unwrap_or_default();
            format!("{} days", duration.as_secs() / 86400)
        })
        .unwrap_or_else(|_| "Unknown".into());

    let (gpu, gpu_driver) = get_gpu_info();

    let user = std::env::var("USER").unwrap_or_else(|_| "user".into());
    let host = System::host_name().unwrap_or_default();
    let os = System::name().unwrap_or_default();
    let kernel = System::kernel_version().unwrap_or_default();

    let displays = detect_displays();

    let uptime_secs = System::uptime();
    let uptime = format!("{}h {}m", uptime_secs / 3600, (uptime_secs % 3600) / 60);

    let packages = detect_packages();

    let wm = std::env::var("XDG_CURRENT_DESKTOP")
        .or_else(|_| std::env::var("DESKTOP_SESSION"))
        .or_else(|_| std::env::var("WAYLAND_DISPLAY").map(|_| "Wayland".to_string()))
        .unwrap_or_else(|_| "Unknown".to_string());

    let mem_used_b = sys.used_memory();
    let mem_total_b = sys.total_memory();

    let disks = Disks::new_with_refreshed_list();
    let mut disk_used_b = 0;
    let mut disk_total_b = 0;

    if let Some(disk) = disks
        .iter()
        .find(|d| d.mount_point().to_string_lossy() == "/")
    {
        disk_total_b = disk.total_space();
        disk_used_b = disk_total_b.saturating_sub(disk.available_space());
    }

    SysData {
        user,
        host,
        os,
        os_age,
        kernel,
        uptime,
        cpu,
        packages,
        wm,
        displays,
        gpu,
        gpu_driver,
        disk_used_b,
        disk_total_b,
        mem_total_b,
        mem_used_b,
    }
}
