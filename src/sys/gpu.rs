pub fn get_gpu_info() -> (String, String) {
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
