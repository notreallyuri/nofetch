pub fn visible_width(text: &str) -> usize {
    let bytes = text.as_bytes();
    let mut count = 0;
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\x1b' && bytes.get(i + 1) == Some(&b'[') {
            i += 2;
            while i < bytes.len() && !bytes[i].is_ascii_alphabetic() {
                i += 1;
            }
            i += 1;
        } else {
            if bytes[i] & 0b1100_0000 != 0b1000_0000 {
                count += 1;
            }
            i += 1;
        }
    }
    count
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
