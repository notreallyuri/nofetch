fn count_packages(path: &str, is_dir_count: bool) -> Result<usize, std::io::Error> {
    if is_dir_count {
        let count = std::fs::read_dir(path)?
            .filter_map(Result::ok)
            .filter(|e| e.path().is_dir())
            .count();
        Ok(count)
    } else {
        let content = std::fs::read_to_string(path)?;
        let count = if path.contains("dpkg") {
            content
                .lines()
                .filter(|l| l.starts_with("Package:"))
                .count()
        } else if path.contains("apk") {
            content.lines().filter(|l| !l.trim().is_empty()).count()
        } else {
            0
        };
        Ok(count)
    }
}

pub fn detect_packages() -> String {
    let package_managers = [
        ("/var/lib/pacman/local", "pacman", true),
        ("/var/lib/dpkg/status", "dpkg", false),
        ("/var/lib/rpm", "rpm", true),
        ("/var/db/pkg", "emerge", true),
        ("/var/lib/apk/installed", "apk", false),
        ("/var/db/xbps", "xbps", true),
        ("/nix/var/nix/profiles/system", "nix", true),
    ];

    for (path, name, is_dir_count) in &package_managers {
        if let Ok(count) = count_packages(path, *is_dir_count) {
            return format!("{} ({})", count, name);
        }
    }

    #[cfg(target_os = "macos")]
    {
        if let Ok(output) = std::process::Command::new("brew")
            .args(["list", "--formula"])
            .output()
        {
            if let Ok(text) = String::from_utf8(output.stdout) {
                let count = text.lines().filter(|l| !l.trim().is_empty()).count();
                if count > 0 {
                    return format!("{} (brew)", count);
                }
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        if let Ok(output) = std::process::Command::new("choco")
            .args(["list", "--local-only"])
            .output()
        {
            if let Ok(text) = String::from_utf8(output.stdout) {
                let count = text
                    .lines()
                    .filter(|l| !l.trim().is_empty() && !l.contains("packages installed"))
                    .count()
                    .saturating_sub(1); // Subtract header line
                if count > 0 {
                    return format!("{} (choco)", count);
                }
            }
        }
    }

    "Unknown".to_string()
}
