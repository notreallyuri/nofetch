use crate::schema::FetchSchema;
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

pub fn load_ascii(art_name: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let config_dir = get_config_path();
    let ascii_dir = config_dir.join("ascii");

    let paths = [
        ascii_dir.join(art_name),
        ascii_dir.join("logos").join(art_name.to_lowercase()),
    ];

    let re = ANSI_REGEX.get_or_init(|| Regex::new(r"\x1B\[[0-9;?]*[a-zA-Z]").unwrap());

    for path in paths {
        if let Ok(content) = std::fs::read_to_string(&path) {
            let cleaned_content = re.replace_all(content.trim_end(), "");

            let lines: Vec<String> = cleaned_content.lines().map(|s| s.to_string()).collect();

            if !lines.is_empty() {
                return Ok(lines);
            }
        }
    }
    Err("Art not found".into())
}

pub fn load_schema(schema_name: &str) -> Result<FetchSchema, String> {
    let schema_path = get_config_path()
        .join("schemas")
        .join(format!("{}.json", schema_name));

    let json = fs::read_to_string(&schema_path).map_err(|_| "Not found".to_string())?;

    serde_json::from_str(&json).map_err(|e| e.to_string())
}
