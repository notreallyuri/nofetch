use viuer::{Config, print_from_file};

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

pub fn draw_with_image(image_path: &std::path::Path, info_lines: &[String]) {
    let conf = Config {
        transparent: true,
        absolute_offset: false,
        x: 2,
        y: 0,
        width: Some(30),
        ..Default::default()
    };

    let expected_img_rows = match image::image_dimensions(image_path) {
        Ok((w, h)) => {
            let width_cells = 30.0;
            ((width_cells * (h as f32) / (w as f32)) / 2.0).ceil() as usize
        }
        Err(_) => 20,
    };

    let safe_buffer = std::cmp::max(info_lines.len(), expected_img_rows) + 1;

    print!("{}", "\n".repeat(safe_buffer));
    print!("\x1b[{}A\r", safe_buffer);

    print!("\x1b[s");

    match print_from_file(image_path, &conf) {
        Ok((img_width, img_height)) => {
            print!("\x1b[u");

            let max_lines = std::cmp::max(img_height as usize, info_lines.len());
            let gap_between_columns = 4;

            for i in 0..max_lines {
                print!("\x1b[{}C", img_width + gap_between_columns);

                if let Some(line) = info_lines.get(i) {
                    println!("{}", line);
                } else {
                    println!();
                }
            }
        }
        Err(e) => {
            print!("\x1b[u");
            eprintln!("Error rendering image: {}", e);
            for line in info_lines {
                println!("  {}", line);
            }
        }
    }
}
