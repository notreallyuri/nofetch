use sysinfo::System;

pub struct SysData {
    pub user: String,
    pub host: String,
    pub os: String,
    pub kernel: String,
    pub uptime: String,
    pub mem_str: String,
    pub cpu: String,
    pub packages: String,
    pub wm: String,
    pub displays: Vec<String>,
    pub gpu: String,
    pub gpu_driver: String,
}

fn get_gpu_info() -> (String, String) {
    #[cfg(target_os = "linux")]
    {
        let lspci_out = std::process::Command::new("lspci")
            .arg("-k")
            .output()
            .ok()
            .and_then(|out| String::from_utf8(out.stdout).ok())
            .unwrap_or_default();

        let mut gpu = "Unknown GPU".to_string();
        let mut gpu_driver = "Unknown".to_string();

        let mut lines = lspci_out.lines().peekable();
        while let Some(line) = lines.next() {
            if line.contains("VGA") || line.contains("3D") {
                gpu = line
                    .split('[')
                    .next_back()
                    .and_then(|s| s.split(']').next())
                    .unwrap_or("Unknown GPU")
                    .to_string();

                while let Some(&next_line) = lines.peek() {
                    if next_line.starts_with('\t') || next_line.starts_with("  ") {
                        if next_line.contains("Kernel driver in use:") {
                            gpu_driver = next_line
                                .split(':')
                                .next_back()
                                .unwrap_or("")
                                .trim()
                                .to_string();
                        }
                        lines.next();
                    } else {
                        break;
                    }
                }
                break;
            }
        }

        if gpu.starts_with("Radeon") {
            gpu = format!("AMD {} [Discrete]", gpu);
        }

        (gpu, gpu_driver)
    }

    #[cfg(target_os = "macos")]
    {
        let out = std::process::Command::new("system_profiler")
            .arg("SPDisplaysDataType")
            .output()
            .ok()
            .and_then(|out| String::from_utf8(out.stdout).ok())
            .unwrap_or_default();

        let mut gpu = "Unknown GPU".to_string();
        for line in out.lines() {
            if line.contains("Chipset Model:") {
                gpu = line.split(':').next_back().unwrap_or("").trim().to_string();
                break;
            }
        }

        (gpu, "Metal".to_string())
    }

    #[cfg(target_os = "windows")]
    {
        let out = std::process::Command::new("wmic")
            .args(["path", "win32_VideoController", "get", "name"])
            .output()
            .ok()
            .and_then(|out| String::from_utf8(out.stdout).ok())
            .unwrap_or_default();

        let gpu = out
            .lines()
            .nth(1)
            .unwrap_or("Unknown GPU")
            .trim()
            .to_string();

        (gpu, "WDDM".to_string())
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        ("Unknown GPU".to_string(), "Unknown".to_string())
    }
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

    let (gpu, gpu_driver) = get_gpu_info();

    let user = std::env::var("USER").unwrap_or_else(|_| "user".into());
    let host = System::host_name().unwrap_or_default();
    let os = System::name().unwrap_or_default();
    let kernel = System::kernel_version().unwrap_or_default();

    let mut displays = Vec::new();

    if std::env::var("WAYLAND_DISPLAY").is_ok()
        && let Ok(output) = std::process::Command::new("hyprctl")
            .args(["monitors", "-j"])
            .output()
        && let Ok(json) = serde_json::from_slice::<serde_json::Value>(&output.stdout)
        && let Some(monitors) = json.as_array()
    {
        for mon in monitors {
            let width = mon["width"].as_i64().unwrap_or(0);
            let height = mon["height"].as_i64().unwrap_or(0);
            let hz = mon["refreshRate"].as_f64().unwrap_or(0.0).round();

            displays.push(format!("{}x{} @ {}Hz [External]", width, height, hz));
        }
    }

    let uptime_secs = System::uptime();
    let uptime = format!("{}h {}m", uptime_secs / 3600, (uptime_secs % 3600) / 60);

    let used_b = sys.used_memory();
    let total_b = sys.total_memory();
    let used_gib = used_b as f64 / (1024.0 * 1024.0 * 1024.0);
    let total_gib = total_b as f64 / (1024.0 * 1024.0 * 1024.0);
    let mem_perc = (used_b as f64 / total_b as f64) * 100.0;
    let mem_str = format!(
        "{:.2} GiB / {:.2} GiB ({:.0}%)",
        used_gib, total_gib, mem_perc
    );

    if displays.is_empty() {
        displays.push("Unknown Display".to_string());
    }

    let packages = std::fs::read_dir("/var/lib/pacman/local")
        .map(|entries| {
            let count = entries
                .filter_map(Result::ok)
                .filter(|e| e.path().is_dir()) // Only count directories
                .count();
            format!("{} (pacman)", count)
        })
        .unwrap_or_else(|_| "Unknown".to_string());

    let wm = std::env::var("XDG_CURRENT_DESKTOP")
        .or_else(|_| std::env::var("DESKTOP_SESSION"))
        .or_else(|_| std::env::var("WAYLAND_DISPLAY").map(|_| "Wayland".to_string()))
        .unwrap_or_else(|_| "Unknown".to_string());

    SysData {
        user,
        host,
        os,
        kernel,
        uptime,
        mem_str,
        cpu,
        packages,
        wm,
        displays,
        gpu,
        gpu_driver,
    }
}
