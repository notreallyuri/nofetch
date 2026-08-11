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
        windows_gpu::get()
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        ("Unknown GPU".to_string(), "Unknown".to_string())
    }
}

/// Windows keeps one registry key per display adapter under the display class
/// GUID, numbered 0000, 0001, … Which number the card you are looking at got is
/// down to install order, so instead of guessing we ask the desktop which
/// adapter is drawing it: EnumDisplayDevicesW hands back that adapter's own
/// registry path in DeviceKey.
#[cfg(target_os = "windows")]
mod windows_gpu {
    use std::ffi::c_void;
    use windows_sys::Win32::Graphics::Gdi::{DISPLAY_DEVICEW, EnumDisplayDevicesW};
    use windows_sys::Win32::System::Registry::{HKEY_LOCAL_MACHINE, RRF_RT_REG_SZ, RegGetValueW};

    const ATTACHED_TO_DESKTOP: u32 = 0x0000_0001;
    const PRIMARY_DEVICE: u32 = 0x0000_0004;
    const ERROR_SUCCESS: u32 = 0;
    const CLASS_KEY: &str =
        r"SYSTEM\CurrentControlSet\Control\Class\{4d36e968-e325-11ce-bfc1-08002be10318}";
    const BASIC_ADAPTER: &str = "microsoft basic display adapter";

    pub fn get() -> (String, String) {
        if let Some((name, subkey)) = active_adapter() {
            let desc = reg_string(&subkey, "DriverDesc").unwrap_or(name);
            let version = reg_string(&subkey, "DriverVersion").unwrap_or_else(|| "WDDM".to_string());
            return (desc, version);
        }
        scan_class_keys()
    }

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    fn from_wide(buf: &[u16]) -> String {
        let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
        String::from_utf16_lossy(&buf[..end]).trim().to_string()
    }

    fn reg_string(subkey: &str, value: &str) -> Option<String> {
        let subkey = wide(subkey);
        let value = wide(value);
        let mut size: u32 = 0;

        unsafe {
            // first call sizes the buffer, second one fills it
            if RegGetValueW(
                HKEY_LOCAL_MACHINE,
                subkey.as_ptr(),
                value.as_ptr(),
                RRF_RT_REG_SZ,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                &mut size,
            ) != ERROR_SUCCESS
                || size == 0
            {
                return None;
            }

            let mut buf = vec![0u16; (size as usize).div_ceil(2)];
            if RegGetValueW(
                HKEY_LOCAL_MACHINE,
                subkey.as_ptr(),
                value.as_ptr(),
                RRF_RT_REG_SZ,
                std::ptr::null_mut(),
                buf.as_mut_ptr() as *mut c_void,
                &mut size,
            ) != ERROR_SUCCESS
            {
                return None;
            }

            let s = from_wide(&buf);
            (!s.is_empty()).then_some(s)
        }
    }

    /// `\Registry\Machine\System\…\0001` -> `System\…\0001`, keeping the case
    /// the registry gave us.
    fn device_key_to_subkey(device_key: &str) -> Option<String> {
        const PREFIX: &str = r"\registry\machine\";
        let rest = device_key.to_ascii_lowercase();
        let rest = rest.strip_prefix(PREFIX)?;
        Some(device_key[device_key.len() - rest.len()..].to_string())
    }

    /// The adapter drawing the desktop, preferring the primary one. Returns its
    /// name and the registry subkey holding its driver details.
    fn active_adapter() -> Option<(String, String)> {
        let mut fallback = None;

        unsafe {
            let mut i = 0;
            let mut device: DISPLAY_DEVICEW = std::mem::zeroed();
            device.cb = std::mem::size_of::<DISPLAY_DEVICEW>() as u32;

            while EnumDisplayDevicesW(std::ptr::null(), i, &mut device, 0) != 0 {
                i += 1;

                if device.StateFlags & ATTACHED_TO_DESKTOP == 0 {
                    continue;
                }
                let Some(subkey) = device_key_to_subkey(&from_wide(&device.DeviceKey)) else {
                    continue;
                };
                let found = (from_wide(&device.DeviceString), subkey);

                if device.StateFlags & PRIMARY_DEVICE != 0 {
                    return Some(found);
                }
                fallback.get_or_insert(found);
            }
        }

        fallback
    }

    /// Nothing is attached to the desktop — a service, an RDP session, a
    /// headless box. Walk the class keys and take the first adapter that isn't
    /// the placeholder driver Windows installs when it has nothing better.
    fn scan_class_keys() -> (String, String) {
        for i in 0..16 {
            let subkey = format!("{}\\{:04}", CLASS_KEY, i);
            let Some(desc) = reg_string(&subkey, "DriverDesc") else {
                continue;
            };
            if desc.to_ascii_lowercase() == BASIC_ADAPTER {
                continue;
            }
            let version = reg_string(&subkey, "DriverVersion").unwrap_or_else(|| "WDDM".to_string());
            return (desc, version);
        }

        ("Unknown GPU".to_string(), "WDDM".to_string())
    }
}

#[cfg(target_os = "linux")]
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

#[cfg(target_os = "linux")]
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

#[cfg(target_os = "linux")]
fn clean_gpu_name(name: &str) -> String {
    if let Some(start) = name.find('[')
        && let Some(end) = name.find(']')
    {
        return name[start + 1..end].to_string();
    }
    name.to_string()
}

#[cfg(target_os = "linux")]
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
