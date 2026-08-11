pub fn detect_displays() -> Vec<String> {
    let mut displays = Vec::new();

    if std::env::var("WAYLAND_DISPLAY").is_ok() {
        if let Ok(output) = std::process::Command::new("hyprctl")
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

        if displays.is_empty()
            && let Ok(output) = std::process::Command::new("swaymsg")
                .args(["-t", "get_outputs", "-r"])
                .output()
            && let Ok(json) = serde_json::from_slice::<serde_json::Value>(&output.stdout)
            && let Some(outputs) = json.as_array()
        {
            for output in outputs {
                if let Some(current_mode) = output["current_mode"].as_object() {
                    let width = current_mode["width"].as_i64().unwrap_or(0);
                    let height = current_mode["height"].as_i64().unwrap_or(0);
                    let hz =
                        (current_mode["refresh"].as_i64().unwrap_or(0) as f64 / 1000.0).round();
                    displays.push(format!("{}x{} @ {}Hz [External]", width, height, hz));
                }
            }
        }

        if displays.is_empty()
            && let Ok(output) = std::process::Command::new("wlr-randr").output()
            && let Ok(text) = String::from_utf8(output.stdout)
        {
            let mut current_width: Option<i32> = None;
            let mut current_height: Option<i32> = None;
            let mut current_hz: Option<i32> = None;

            for line in text.lines() {
                let trimmed = line.trim();
                if trimmed.contains("current") {
                    if let Some(res_part) = trimmed.split("px").next()
                        && let Some((w, h)) = res_part.trim().split_once('x')
                    {
                        current_width = w.trim().parse().ok();
                        current_height = h.trim().parse().ok();
                    }
                    if let Some(hz_part) = trimmed.split("Hz").next()
                        && let Some(hz_str) = hz_part.split(',').nth(1)
                    {
                        current_hz = hz_str.trim().parse::<f64>().ok().map(|f| f.round() as i32);
                    }

                    if let (Some(w), Some(h), Some(hz)) =
                        (current_width, current_height, current_hz)
                    {
                        displays.push(format!("{}x{} @ {}Hz [External]", w, h, hz));
                        current_width = None;
                        current_height = None;
                        current_hz = None;
                    }
                }
            }
        }
    }

    if displays.is_empty()
        && std::env::var("DISPLAY").is_ok()
        && let Ok(output) = std::process::Command::new("xrandr").output()
        && let Ok(text) = String::from_utf8(output.stdout)
    {
        for line in text.lines() {
            if line.contains(" connected") && line.contains("*") {
                for part in line.split_whitespace() {
                    if part.contains('x')
                        && !part.contains("connected")
                        && let Some((w, h_part)) = part.split_once('x')
                    {
                        let h: String = h_part.chars().take_while(|c| c.is_numeric()).collect();
                        if let (Ok(width), Ok(height)) = (w.parse::<i32>(), h.parse::<i32>()) {
                            let hz = line
                                .split_whitespace()
                                .find(|s| s.contains('*'))
                                .and_then(|s| s.trim_end_matches('*').parse::<f64>().ok())
                                .map(|f| f.round() as i32)
                                .unwrap_or(60);

                            displays.push(format!("{}x{} @ {}Hz [External]", width, height, hz));
                            break;
                        }
                    }
                }
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        if displays.is_empty()
            && let Ok(output) = std::process::Command::new("system_profiler")
                .arg("SPDisplaysDataType")
                .output()
            && let Ok(text) = String::from_utf8(output.stdout)
        {
            for line in text.lines() {
                if line.contains("Resolution:") {
                    if let Some(res) = line.split(':').nth(1) {
                        displays.push(res.trim().to_string());
                    }
                }
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        use std::mem;
        use windows_sys::Win32::Graphics::Gdi::{
            DEVMODEW, DISPLAY_DEVICEW, ENUM_CURRENT_SETTINGS, EnumDisplayDevicesW,
            EnumDisplaySettingsW,
        };

        if displays.is_empty() {
            unsafe {
                let mut i = 0;
                let mut display_device: DISPLAY_DEVICEW = mem::zeroed();
                display_device.cb = mem::size_of::<DISPLAY_DEVICEW>() as u32;

                while EnumDisplayDevicesW(std::ptr::null(), i, &mut display_device, 0) != 0 {
                    if (display_device.StateFlags & 1) != 0 {
                        let mut dev_mode: DEVMODEW = mem::zeroed();
                        dev_mode.dmSize = mem::size_of::<DEVMODEW>() as u16;

                        if EnumDisplaySettingsW(
                            display_device.DeviceName.as_ptr(),
                            ENUM_CURRENT_SETTINGS,
                            &mut dev_mode,
                        ) != 0
                        {
                            displays.push(format!(
                                "{}x{} @ {}Hz",
                                dev_mode.dmPelsWidth,
                                dev_mode.dmPelsHeight,
                                dev_mode.dmDisplayFrequency
                            ));
                        }
                    }
                    i += 1;
                }
            }
        }
    }

    if displays.is_empty() {
        displays.push("Unknown Display".to_string());
    }

    displays
}
