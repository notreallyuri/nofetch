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
    let config_dir = get_config_path().join(subfolder);

    match fs::read_dir(config_dir) {
        Ok(entries) => entries
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().to_string())
            .map(|s| s.split('.').next().unwrap_or(&s).to_string())
            .collect(),
        Err(_) => vec![],
    }
}

static ANSI_REGEX: OnceLock<Regex> = OnceLock::new();
static ASCII_COLOR_REGEX: OnceLock<Regex> = OnceLock::new();

pub fn colorize_ascii(raw_lines: Vec<String>, palette: &[FetchColor]) -> Vec<String> {
    let re = ASCII_COLOR_REGEX.get_or_init(|| Regex::new(r"\$\{?c?([0-9]+)\}?").unwrap());

    let total_lines = raw_lines.len();
    let has_tokens = raw_lines.iter().any(|l| re.is_match(l));

    raw_lines
        .into_iter()
        .enumerate()
        .map(|(i, line)| {
            if has_tokens {
                let colored_line = re.replace_all(&line, |caps: &regex::Captures| {
                    if let Ok(index) = caps[1].parse::<usize>() {
                        // Arrays are 0-indexed, tokens are 1-indexed
                        if index > 0 && index <= palette.len() {
                            return palette[index - 1].to_ansi_code().to_string();
                        }
                    }
                    "".to_string()
                });

                format!("{}\x1b[0m", colored_line)
            } else if !palette.is_empty() {
                let color_index = (i * palette.len()) / total_lines;
                let current_color = &palette[color_index];

                current_color.apply(&line).to_string()
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
