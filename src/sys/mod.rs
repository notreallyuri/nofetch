use std::{
    collections::{HashMap, HashSet},
    thread,
    time::SystemTime,
};
use sysinfo::{CpuRefreshKind, Disks, System};

mod displays;
mod gpu;
mod packages;

use crate::{
    schema::{kind::StatKind, module::Module},
    sys::{displays::detect_displays, gpu::get_gpu_info, packages::detect_packages},
};

pub struct SysData {
    pub user: String,
    pub host: String,
    pub os: String,
    pub os_age: String,
    pub kernel: String,
    pub uptime: String,
    pub shell: String,

    pub mem_used_b: u64,
    pub mem_total_b: u64,
    pub disks: HashMap<String, (u64, u64)>,

    pub cpu: String,
    pub packages: String,
    pub wm: String,
    pub displays: Vec<String>,
    pub gpu: String,
    pub gpu_driver: String,
}

pub fn gather_info(modules: &[Module]) -> SysData {
    let mut needs_cpu = false;
    let mut needs_mem = false;
    let mut needs_gpu = false;
    let mut needs_pkgs = false;
    let mut needs_disp = false;
    let mut disk_targets = HashSet::new();

    for m in modules {
        if let Module::Stat(stat) = m {
            match stat.kind {
                StatKind::Cpu => needs_cpu = true,
                StatKind::Memory => needs_mem = true,
                StatKind::Gpu | StatKind::GpuDriver => needs_gpu = true,
                StatKind::Packages => needs_pkgs = true,
                StatKind::Display => needs_disp = true,
                StatKind::Disk => {
                    let default_path = if cfg!(target_os = "windows") {
                        "C:\\"
                    } else {
                        "/"
                    };
                    disk_targets.insert(stat.path.as_deref().unwrap_or(default_path).to_string());
                }
                _ => {}
            }
        }
    }

    let mut sys = System::new();

    if needs_cpu {
        sys.refresh_cpu_specifics(CpuRefreshKind::everything());
    }
    if needs_mem {
        sys.refresh_memory();
    }

    let user = std::env::var("USER").unwrap_or_else(|_| "user".into());
    let host = System::host_name().unwrap_or_default();
    let os = System::name().unwrap_or_default();
    let kernel = System::kernel_version().unwrap_or_default();
    let uptime_secs = System::uptime();
    let uptime = format!("{}h {}m", uptime_secs / 3600, (uptime_secs % 3600) / 60);

    let mem_used_b = sys.used_memory();
    let mem_total_b = sys.total_memory();

    let cpu = if needs_cpu {
        let live_mhz = sys.cpus().first().map(|c| c.frequency()).unwrap_or(0);
        let live_ghz = live_mhz as f64 / 1000.0;
        sys.cpus()
            .first()
            .map(|c| {
                let brand = c
                    .brand()
                    .trim()
                    .replace(" 6-Core Processor", "")
                    .replace(" 12-Thread Processor", "");
                format!("{} @ {:.2} GHz", brand, live_ghz)
            })
            .unwrap_or_else(|| "Unknown CPU".into())
    } else {
        "".into()
    };

    let os_age = std::fs::metadata("/")
        .and_then(|m| m.created())
        .map(|created| {
            let secs = SystemTime::now()
                .duration_since(created)
                .unwrap_or_default()
                .as_secs();
            format!("{} days", secs / 86400)
        })
        .unwrap_or_else(|_| "Unknown".into());

    let wm = std::env::var("XDG_CURRENT_DESKTOP")
        .or_else(|_| std::env::var("DESKTOP_SESSION"))
        .unwrap_or_else(|_| {
            if cfg!(target_os = "windows") {
                "Explorer".to_string()
            } else {
                "Unknown".to_string()
            }
        });

    let shell = std::env::var("SHELL")
        .ok()
        .and_then(|s| {
            std::path::Path::new(&s)
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
        })
        .unwrap_or_else(|| {
            if cfg!(target_os = "windows") {
                "CMD/PS".to_string()
            } else {
                "Unknown".to_string()
            }
        });

    let (gpu, gpu_driver, displays, packages) = thread::scope(|s| {
        let gpu_t = needs_gpu.then(|| s.spawn(get_gpu_info));
        let display_t = needs_disp.then(|| s.spawn(detect_displays));
        let pkg_t = needs_pkgs.then(|| s.spawn(detect_packages));

        let (gpu, gpu_driver) = gpu_t
            .map(|t| {
                t.join()
                    .unwrap_or_else(|_| ("Unknown GPU".into(), "Unknown".into()))
            })
            .unwrap_or_else(|| ("".into(), "".into()));
        let displays = display_t
            .map(|t| t.join().unwrap_or_default())
            .unwrap_or_default();
        let packages = pkg_t
            .map(|t| t.join().unwrap_or_else(|_| "".into()))
            .unwrap_or_else(|| "".into());

        (gpu, gpu_driver, displays, packages)
    });

    let mut disks = HashMap::new();
    if !disk_targets.is_empty() {
        let sys_disks = Disks::new_with_refreshed_list();
        for target in disk_targets {
            let best_disk = sys_disks
                .iter()
                .filter(|d| {
                    let mount = d.mount_point().to_string_lossy();
                    if cfg!(target_os = "windows") {
                        mount.starts_with(&target)
                    } else {
                        target.starts_with(mount.as_ref())
                    }
                })
                .max_by_key(|d| d.mount_point().to_string_lossy().len());

            if let Some(d) = best_disk {
                disks.insert(
                    target,
                    (
                        d.total_space().saturating_sub(d.available_space()),
                        d.total_space(),
                    ),
                );
            }
        }
    }

    SysData {
        user,
        host,
        os,
        os_age,
        kernel,
        shell,
        uptime,
        cpu,
        packages,
        wm,
        displays,
        gpu,
        gpu_driver,
        disks,
        mem_total_b,
        mem_used_b,
    }
}
