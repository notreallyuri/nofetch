use crate::schema::FetchColor;

pub fn draw(logo: &[String], info_lines: &[String], art_color: Option<&FetchColor>) {
    let logo_width = logo.iter().map(|s| s.chars().count()).max().unwrap_or(0);

    let max_lines = std::cmp::max(logo.len(), info_lines.len());

    for i in 0..max_lines {
        let left_raw = logo.get(i).map(|s| s.as_str()).unwrap_or("");
        let right = info_lines.get(i).map(|s| s.as_str()).unwrap_or("");

        let padded_left = format!("{:<width$}", left_raw, width = logo_width);

        let final_left = match art_color {
            Some(color) => color.apply(&padded_left).to_string(),
            None => padded_left,
        };

        println!("  {}   {}", final_left, right);
    }
}
