use crate::schema::{FetchArt, FetchColor, FetchComponent, FetchModule, FetchSchema, WidthMode};
use directories::ProjectDirs;
use mlua::{Lua, Table};
use regex::Regex;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::OnceLock,
};

pub fn get_config_path() -> PathBuf {
    ProjectDirs::from("", "", "nothings")
        .map(|d| d.config_dir().to_path_buf())
        .expect("Could not find config directory")
}

pub fn list_available_configs(subfolder: &str) -> Vec<String> {
    let base_path = get_config_path().join(subfolder);
    let mut available = Vec::new();

    let mut scan_dir = |path: std::path::PathBuf| {
        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                if let Ok(file_type) = entry.file_type()
                    && file_type.is_file()
                {
                    let name = entry.file_name().to_string_lossy().to_string();
                    let clean_name = name
                        .strip_suffix(".txt")
                        .or_else(|| name.strip_suffix(".json"))
                        .or_else(|| name.strip_suffix(".lua"))
                        .unwrap_or(&name)
                        .to_string();
                    available.push(clean_name);
                }
            }
        }
    };

    scan_dir(base_path.clone());

    if subfolder == "arts" {
        scan_dir(base_path.join("logos"));
    }

    available.sort();
    available.dedup();
    available
}

static ANSI_REGEX: OnceLock<Regex> = OnceLock::new();

pub fn colorize_ascii(raw_lines: Vec<String>, palette: &[FetchColor]) -> Vec<String> {
    let total_lines = raw_lines.len();

    raw_lines
        .into_iter()
        .enumerate()
        .map(|(i, line)| {
            if !palette.is_empty() {
                let color_index = (i * palette.len()) / total_lines;
                palette[color_index].apply(&line).to_string()
            } else {
                line
            }
        })
        .collect()
}

pub fn load_ascii(art_name: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let config_dir = get_config_path();
    let ascii_dir = config_dir.join("arts");

    let clean_name = art_name.trim();
    let lower_name = clean_name.to_lowercase();

    let paths = [
        ascii_dir.join(clean_name),
        ascii_dir.join(format!("{}.txt", clean_name)),
        ascii_dir.join("logos").join(&lower_name),
        ascii_dir.join("logos").join(format!("{}.txt", lower_name)),
    ];

    let re = ANSI_REGEX.get_or_init(|| Regex::new(r"\x1B\[[0-9;?]*[a-zA-Z]").unwrap());

    for path in paths {
        if let Ok(bytes) = std::fs::read(&path) {
            let content = String::from_utf8_lossy(&bytes);

            let cleaned_content = re.replace_all(content.trim_end(), "");
            let lines: Vec<String> = cleaned_content.lines().map(|s| s.to_string()).collect();

            if !lines.is_empty() {
                return Ok(lines);
            }
        }
    }

    Err(format!("Art '{}' not found", clean_name).into())
}

pub fn is_image(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|s| s.to_str()),
        Some("png") | Some("jpg") | Some("jpeg") | Some("webp") | Some("gif")
    )
}

pub fn load_config(config_name: &str) -> Result<FetchSchema, String> {
    let config_dir = get_config_path();
    let candidates = [
        config_dir.join(format!("{}.lua", config_name)),
        config_dir.join("config.lua"),
    ];

    let path = candidates
        .iter()
        .find(|p| p.exists())
        .ok_or_else(|| format!("No config found (tried {}.lua, config.lua)", config_name))?;

    let lua = Lua::new();
    let chunk = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let table: Table = lua.load(&chunk).eval().map_err(|e| e.to_string())?;

    parse_schema_from_lua(table).map_err(|e| e.to_string())
}

fn parse_schema_from_lua(table: Table) -> Result<FetchSchema, mlua::Error> {
    let art = if let Ok(art_table) = table.get::<Table>("art") {
        let name: Option<String> = art_table.get("name").ok();
        let colors = if let Ok(colors_table) = art_table.get::<Table>("colors") {
            colors_table
                .sequence_values::<String>()
                .filter_map(|v| v.ok())
                .filter_map(|s| FetchColor::from_str_name(&s))
                .collect()
        } else {
            vec![]
        };
        Some(FetchArt {
            name,
            colors: if colors.is_empty() {
                None
            } else {
                Some(colors)
            },
        })
    } else {
        None
    };

    let modules_table: Table = table.get("modules")?;
    let mut modules = Vec::new();

    for module_val in modules_table.sequence_values::<Table>() {
        let m = module_val?;
        let kind_str: String = m.get("type")?;
        let kind = parse_component(&kind_str).ok_or_else(|| {
            mlua::Error::RuntimeError(format!("Unknown module type: '{}'", kind_str))
        })?;

        modules.push(FetchModule {
            kind,
            label: m.get("label").ok(),
            value: m.get("value").ok(),
            icon: m.get("icon").ok(),
            symbol: m.get("symbol").ok(),
            format: m.get("format").ok(),
            fill: m.get("fill").ok(),
            color: m
                .get::<String>("color")
                .ok()
                .and_then(|s| FetchColor::from_str_name(&s)),
            width: m
                .get::<String>("width")
                .ok()
                .and_then(|s| match s.as_str() {
                    "full" => Some(WidthMode::Full),
                    "fit" => Some(WidthMode::Fit),
                    _ => None,
                }),
            thresholds: m
                .get::<Table>("thresholds")
                .ok()
                .map(|t| t.sequence_values::<f64>().filter_map(|v| v.ok()).collect()),
        });
    }

    Ok(FetchSchema { art, modules })
}

fn parse_component(s: &str) -> Option<FetchComponent> {
    match s {
        "os" => Some(FetchComponent::Os),
        "title" => Some(FetchComponent::Title),
        "os_age" => Some(FetchComponent::OsAge),
        "kernel" => Some(FetchComponent::Kernel),
        "uptime" => Some(FetchComponent::Uptime),
        "memory" => Some(FetchComponent::Memory),
        "cpu" => Some(FetchComponent::Cpu),
        "colors" => Some(FetchComponent::Colors),
        "custom" => Some(FetchComponent::Custom),
        "packages" => Some(FetchComponent::Packages),
        "wm" => Some(FetchComponent::Wm),
        "display" => Some(FetchComponent::Display),
        "gpu" => Some(FetchComponent::Gpu),
        "gpu_driver" => Some(FetchComponent::GpuDriver),
        "disk" => Some(FetchComponent::Disk),
        "shell" => Some(FetchComponent::Shell),
        _ => None,
    }
}

pub fn get_art_path(art_name: &str) -> Option<std::path::PathBuf> {
    let ascii_dir = get_config_path().join("arts");
    let clean_name = art_name.trim();
    let lower_name = clean_name.to_lowercase();

    let paths = [
        ascii_dir.join(clean_name),
        ascii_dir.join(format!("{}.txt", clean_name)),
        ascii_dir.join(format!("{}.png", clean_name)),
        ascii_dir.join(format!("{}.jpg", clean_name)),
        ascii_dir.join(format!("{}.gif", clean_name)),
        ascii_dir.join("logos").join(&lower_name),
        ascii_dir.join("logos").join(format!("{}.txt", lower_name)),
        ascii_dir.join("logos").join(format!("{}.gif", lower_name)),
    ];

    paths.into_iter().find(|p| p.exists())
}
