pub mod kitty;

use std::path::Path;
use viuer::{Config, print_from_file};

/// Columns the logo is drawn across, and the gap before the info column.
const LOGO_COLS: u32 = 30;
const LEFT_MARGIN: u32 = 2;
const COLUMN_GAP: u32 = 4;

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

/// `animate` is the user's `--no-animation` flag, inverted. When it is off a
/// GIF is drawn as its first frame and nothing loops: no frames are uploaded,
/// and the raw file never reaches a terminal that would animate it on its own.
pub fn draw_with_image(image_path: &Path, info_lines: &[String], animate: bool) {
    // An animated GIF wants whichever protocol lets the *terminal* do the
    // animating, because that is the only kind nofetch can walk away from.
    //
    //   kitty            -> kitty `a=f` frames, handled below
    //   WezTerm, iTerm2,
    //   mintty, rio,
    //   Warp, Konsole    -> iTerm2 OSC 1337, which animates raw GIF bytes
    //                       natively. viuer already speaks it, so hand it over
    //                       rather than drawing a still through kitty.
    //   everything else  -> no native animation anywhere; the kitty path falls
    //                       back to driving the frames itself.
    let is_gif = image_path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("gif"));

    if is_gif && animate && !kitty::detect().animation && viuer::is_iterm_supported() {
        return draw_with_viuer(image_path, info_lines, animate);
    }

    match kitty::place_beside(image_path, LOGO_COLS, LEFT_MARGIN, animate) {
        Ok(placement) => {
            let lines = std::cmp::max(placement.rows as usize, info_lines.len());
            compose(lines, info_lines);
            // Blocks only where nothing else can animate. The info column is
            // already on screen by now, so an interrupt loses nothing.
            placement.animate(lines as u32);
        }
        Err(_) => draw_with_viuer(image_path, info_lines, animate),
    }
}

/// Print `lines` rows of the info column beside an image that has already been
/// placed at the top-left of the block, with the cursor still sitting there.
fn compose(lines: usize, info_lines: &[String]) {
    let text_column = LEFT_MARGIN + LOGO_COLS + COLUMN_GAP;

    for i in 0..lines {
        // Back to the start of the row, then out past the image. Absolute
        // positioning would fight with whatever is already on screen.
        print!("\r\x1b[{}C", text_column);
        println!("{}", info_lines.get(i).map_or("", |s| s.as_str()));
    }
}

/// Sixel, iTerm and half-block terminals still go through viuer — including
/// the iTerm2 GIF path above, which needs viuer's raw-file transmission.
///
/// `use_kitty` is off deliberately: the kitty path is handled above, and
/// leaving viuer's own kitty probe enabled would re-run the blocking terminal
/// query that this module exists to avoid.
///
/// A GIF only gets here with `animate` off once the kitty path has been ruled
/// out, and then it must not be sent as a file: the iTerm2 protocol hands the
/// raw bytes to the terminal, which animates them. Decoding to the first frame
/// and printing that is what makes `--no-animation` hold on those terminals.
fn draw_with_viuer(image_path: &Path, info_lines: &[String], animate: bool) {
    let conf = Config {
        transparent: true,
        absolute_offset: false,
        use_kitty: false,
        x: LEFT_MARGIN as u16,
        y: 0,
        width: Some(LOGO_COLS),
        truecolor: cfg!(target_os = "windows"),
        ..Default::default()
    };

    let expected_img_rows = image::image_dimensions(image_path).map_or(20, |dimensions| {
        kitty::rows_for(LOGO_COLS, dimensions) as usize
    });

    let reserved = std::cmp::max(info_lines.len(), expected_img_rows) + 1;
    print!("{}", "\n".repeat(reserved));
    print!("\x1b[{}A\r", reserved);
    print!("\x1b[s");

    let is_gif = image_path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("gif"));

    // `image::open` on a GIF decodes frame one and stops, which is exactly the
    // still wanted here.
    let still = (is_gif && !animate)
        .then(|| image::open(image_path).ok())
        .flatten();

    let drawn = match &still {
        Some(img) => viuer::print(img, &conf),
        None => print_from_file(image_path, &conf),
    };

    match drawn {
        Ok((img_width, img_height)) => {
            print!("\x1b[u");

            let max_lines = std::cmp::max(img_height as usize, info_lines.len());
            for i in 0..max_lines {
                print!("\x1b[{}C", img_width + COLUMN_GAP);
                println!("{}", info_lines.get(i).map_or("", |s| s.as_str()));
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
