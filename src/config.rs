use crate::schema::{
    Art, Schema,
    color::FetchColor,
    kind::StatKind,
    module::{
        ColorSymbol, ColorsModule, Module, SeparatorModule, StatModule, TextModule, WidthMode,
    },
};
use colored::Colorize;
use mlua::{Lua, Table};
use regex::Regex;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::OnceLock,
};

const DEFAULT_CONFIG: &str = include_str!("../default_nofetch.lua");
const DEFAULT_ART: &str = include_str!("../default_art.txt");

pub fn get_config_path() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        directories::UserDirs::new()
            .map(|u| {
                u.home_dir()
                    .join("AppData")
                    .join("Roaming")
                    .join("nothings")
            })
            .expect("Could not find Roaming AppData")
    }
    #[cfg(not(target_os = "windows"))]
    {
        directories::ProjectDirs::from("", "", "nothings")
            .map(|d| d.config_dir().to_path_buf())
            .expect("Could not find config directory")
    }
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
    if total_lines == 0 {
        return raw_lines;
    }

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

fn clean_ascii(content: &str) -> Vec<String> {
    let re = ANSI_REGEX.get_or_init(|| Regex::new(r"\x1B\[[0-9;?]*[a-zA-Z]").unwrap());
    let cleaned = re.replace_all(content.trim_end(), "");
    cleaned.lines().map(|s| s.to_string()).collect()
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

    for path in paths {
        if let Ok(bytes) = std::fs::read(&path) {
            let lines = clean_ascii(&String::from_utf8_lossy(&bytes));
            if !lines.is_empty() {
                return Ok(lines);
            }
        }
    }

    Err(format!("Art '{}' not found", clean_name).into())
}

/// Built-in OS-neutral logo, used when art was auto-detected but no matching
/// file exists yet (e.g. a fresh install with an unpopulated `arts/` folder).
pub fn default_ascii() -> Vec<String> {
    clean_ascii(DEFAULT_ART)
}

pub fn is_image(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|s| s.to_str()),
        Some("png") | Some("jpg") | Some("jpeg") | Some("webp") | Some("gif")
    )
}

pub fn load_config(config_name: &str) -> Result<Schema, String> {
    let config_dir = get_config_path();
    let candidates = [
        config_dir.join(format!("{}.lua", config_name)),
        config_dir.join("nofetch.lua"),
    ];

    let path = candidates.iter().find(|p| p.exists()).cloned();

    let final_path = match path {
        Some(p) => p,
        None => {
            let default_path = config_dir.join("nofetch.lua");

            if let Err(e) = fs::create_dir_all(&config_dir) {
                return Err(format!("Failed to create config directory: {}", e));
            }

            if let Err(e) = fs::write(&default_path, DEFAULT_CONFIG) {
                return Err(format!("Failed to write default config: {}", e));
            }

            println!(
                "  {} Created default config at {}",
                "󰌵".blue(),
                default_path.display()
            );

            default_path
        }
    };

    let lua = Lua::new();
    let chunk = std::fs::read_to_string(final_path).map_err(|e| e.to_string())?;
    let table: Table = lua.load(&chunk).eval().map_err(|e| e.to_string())?;

    parse_schema_from_lua(table).map_err(|e| e.to_string())
}

fn parse_schema_from_lua(table: Table) -> Result<Schema, mlua::Error> {
    let art = if let Ok(art_table) = table.get::<Table>("art") {
        let name: Option<String> = art_table.get("name").ok();
        let colors: Vec<FetchColor> = art_table
            .get::<Table>("colors")
            .map(|t| {
                t.sequence_values::<String>()
                    .filter_map(|v| v.ok())
                    .filter_map(|s| FetchColor::from_str_name(&s))
                    .collect()
            })
            .unwrap_or_default();

        Some(Art {
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

        let module = match kind_str.as_str() {
            "colors" => Module::Colors(ColorsModule {
                symbol: m
                    .get::<String>("symbol")
                    .ok()
                    .map(|s| s.parse::<ColorSymbol>().unwrap())
                    .unwrap_or(ColorSymbol::Square),
            }),

            "custom" => {
                if let Ok(fill) = m.get::<String>("fill") {
                    Module::Separator(SeparatorModule {
                        fill,
                        value: m.get("value").ok(),
                        width: m
                            .get::<String>("width")
                            .ok()
                            .map(|s| match s.as_str() {
                                "fit" => WidthMode::Fit,
                                _ => WidthMode::Full,
                            })
                            .unwrap_or(WidthMode::Full),
                        color: m
                            .get::<String>("color")
                            .ok()
                            .and_then(|s| FetchColor::from_str_name(&s)),
                    })
                } else {
                    Module::Text(TextModule {
                        value: m.get("value").unwrap_or_default(),
                        color: m
                            .get::<String>("color")
                            .ok()
                            .and_then(|s| FetchColor::from_str_name(&s)),
                    })
                }
            }

            _ => Module::Stat(StatModule {
                kind: parse_stat_kind(&kind_str).ok_or_else(|| {
                    mlua::Error::RuntimeError(format!("Unknown module type: '{}'", kind_str))
                })?,
                path: m.get::<String>("path").ok().filter(|s| !s.is_empty()),
                label: m.get::<String>("label").ok().filter(|s| !s.is_empty()),
                icon: m.get::<String>("icon").ok().filter(|s| !s.is_empty()),
                color: m
                    .get::<String>("color")
                    .ok()
                    .and_then(|s| FetchColor::from_str_name(&s)),
                format: m.get::<String>("format").ok().filter(|s| !s.is_empty()),
                separator: m.get::<String>("separator").ok().filter(|s| !s.is_empty()),
                thresholds: m.get::<Table>("thresholds").ok().and_then(|t| {
                    let v: Vec<f64> = t.sequence_values().filter_map(|v| v.ok()).collect();
                    if v.len() >= 2 {
                        Some([v[0], v[1]])
                    } else {
                        None
                    }
                }),
            }),
        };

        modules.push(module);
    }

    Ok(Schema { art, modules })
}

fn parse_stat_kind(s: &str) -> Option<StatKind> {
    match s {
        "os" => Some(StatKind::Os),
        "title" => Some(StatKind::Title),
        "os_age" => Some(StatKind::OsAge),
        "kernel" => Some(StatKind::Kernel),
        "uptime" => Some(StatKind::Uptime),
        "memory" => Some(StatKind::Memory),
        "cpu" => Some(StatKind::Cpu),
        "packages" => Some(StatKind::Packages),
        "wm" => Some(StatKind::Wm),
        "display" => Some(StatKind::Display),
        "gpu" => Some(StatKind::Gpu),
        "gpu_driver" => Some(StatKind::GpuDriver),
        "disk" => Some(StatKind::Disk),
        "shell" => Some(StatKind::Shell),
        _ => None,
    }
}

pub fn get_art_path(art_name: &str) -> Option<PathBuf> {
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
