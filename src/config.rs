use crate::schema::{FetchColor, FetchSchema};
use directories::ProjectDirs;
use regex::Regex;
use std::{fs, path::PathBuf, sync::OnceLock};

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
                        .unwrap_or(&name)
                        .to_string();
                    available.push(clean_name);
                }
            }
        }
    };

    scan_dir(base_path.clone());

    if subfolder == "ascii" {
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
    let ascii_dir = config_dir.join("ascii");

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

pub fn load_schema(schema_name: &str) -> Result<FetchSchema, String> {
    let schema_path = get_config_path()
        .join("schemas")
        .join(format!("{}.json", schema_name));

    let json = fs::read_to_string(&schema_path).map_err(|_| "Not found".to_string())?;

    serde_json::from_str(&json).map_err(|e| e.to_string())
}
