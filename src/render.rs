use regex::Regex;
use std::sync::OnceLock;

static ANSI_REGEX: OnceLock<Regex> = OnceLock::new();

fn visible_width(text: &str) -> usize {
    let re = ANSI_REGEX.get_or_init(|| Regex::new(r"\x1B\[[0-9;?]*[a-zA-Z]").unwrap());
    re.replace_all(text, "").chars().count()
}

pub fn draw(logo: &[String], info_lines: &[String]) {
    let logo_width = logo.iter().map(|s| visible_width(s)).max().unwrap_or(0);

    let max_lines = std::cmp::max(logo.len(), info_lines.len());

    for i in 0..max_lines {
        let left_raw = logo.get(i).map(|s| s.as_str()).unwrap_or("");
        let right = info_lines.get(i).map(|s| s.as_str()).unwrap_or("");

        let current_width = visible_width(left_raw);
        let padding_needed = logo_width.saturating_sub(current_width);
        let padding = " ".repeat(padding_needed);

        println!("  {}{}   {}", left_raw, padding, right);
    }
}
