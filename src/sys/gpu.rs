pub fn get_gpu_info() -> (String, String) {
    #[cfg(target_os = "linux")]
    {
        if let Ok(entries) = std::fs::read_dir("/sys/class/drm") {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name = name.to_string_lossy();

                if !name.starts_with("card") || name.contains('-') {
                    continue;
                }

                let base = entry.path();
                let device = base.join("device");

                let driver = device.join("driver");
                let gpu_driver = std::fs::read_link(&driver)
                    .ok()
                    .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
                    .unwrap_or_else(|| "Unknown".into());

                let gpu_name =
                    read_gpu_name_from_sysfs(&device).unwrap_or_else(|| "Unknown GPU".into());

                return (gpu_name, gpu_driver);
            }
        }

        get_gpu_lspci()
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

fn read_gpu_name_from_sysfs(device: &std::path::Path) -> Option<String> {
    if let Ok(label) = std::fs::read_to_string(device.join("label")) {
        return Some(label.trim().to_string());
    }

    let uevent = std::fs::read_to_string(device.join("uevent")).ok()?;
    let mut vendor_id: Option<String> = None;
    let mut device_id: Option<String> = None;

    for line in uevent.lines() {
        if let Some(v) = line.strip_prefix("PCI_ID=") {
            if let Some((vendor, device)) = v.split_once(':') {
                vendor_id = Some(vendor.to_ascii_lowercase());
                device_id = Some(device.to_ascii_lowercase());
            }
            break;
        }
    }

    if let (Some(vid), Some(did)) = (vendor_id, device_id) {
        if let Some(name) = lookup_pci_name(&vid, &did) {
            return Some(format!("{} [Discrete]", clean_gpu_name(&name)));
        }
        return Some(format!(
            "GPU [PCI {}:{}]",
            vid.to_uppercase(),
            did.to_uppercase()
        ));
    }

    None
}

fn get_gpu_lspci() -> (String, String) {
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

fn clean_gpu_name(name: &str) -> String {
    if let Some(start) = name.find('[')
        && let Some(end) = name.find(']')
    {
        return name[start + 1..end].to_string();
    }
    name.to_string()
}

fn lookup_pci_name(vendor_id: &str, device_id: &str) -> Option<String> {
    let paths = ["/usr/share/hwdata/pci.ids", "/usr/share/misc/pci.ids"];
    let content = paths.iter().find_map(|p| std::fs::read_to_string(p).ok())?;

    let mut in_vendor = false;
    for line in content.lines() {
        if line.starts_with('#') || line.is_empty() {
            continue;
        }

        if !line.starts_with('\t') {
            in_vendor = line.starts_with(vendor_id);
        } else if in_vendor && line.starts_with('\t') && !line.starts_with("\t\t") {
            let trimmed = line.trim_start_matches('\t');
            if trimmed.starts_with(device_id) {
                return trimmed.split_once("  ").map(|x| x.1.trim().to_string());
            }
        }
    }
    None
}
