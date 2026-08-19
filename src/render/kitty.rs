//! A native kitty graphics protocol backend.
//!
//! viuer can already draw through kitty, but with a fixed feature set that
//! costs more than it gives here: it never sends an image id (so every run
//! leaks another copy into the terminal's image store), it never sends `C=1`
//! (so the caller has to fence the placement with save/restore cursor, which
//! breaks the moment the screen scrolls), it has no animation, and its
//! capability probe blocks on a terminal reply that some terminals never send
//! — the freeze recorded in TODO.md.
//!
//! So the kitty path is ours and viuer keeps the rest (sixel, iTerm, blocks).
//!
//! Protocol reference: <https://sw.kovidgoyal.net/kitty/graphics-protocol/>

use base64::{Engine, engine::general_purpose::STANDARD};
use image::{AnimationDecoder, DynamicImage, codecs::gif::GifDecoder};
use std::collections::hash_map::DefaultHasher;
use std::fs::File;
use std::hash::{Hash, Hasher};
use std::io::{BufReader, IsTerminal, Write};
use std::path::Path;
use std::sync::OnceLock;
use std::time::Duration;

/// Escape codes carry base64 in chunks of at most 4096 bytes.
const CHUNK: usize = 4096;

/// GIFs routinely ask for a 0ms or 10ms gap, meaning "as fast as you can".
/// Browsers clamp those; so do we, or the terminal burns CPU on a logo.
const MIN_GAP_MS: i32 = 20;

/// What this terminal can actually do with the protocol.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Caps {
    /// Understands `\x1b_G` graphics commands at all.
    pub graphics: bool,
    /// Will read pixels from a path we hand it (`t=f`) instead of needing
    /// every byte inline. False over SSH, and on terminals that just don't.
    pub shares_filesystem: bool,
    /// Implements `a=f` frame loading and `a=a` playback. Only kitty itself
    /// does, as of writing. Terminals without it still animate — either
    /// through the iTerm2 protocol, or by nofetch driving the frames — but
    /// they must not be sent frame data they would silently discard.
    pub animation: bool,
}

impl Caps {
    const NONE: Self = Self {
        graphics: false,
        shares_filesystem: false,
        animation: false,
    };
}

/// Detect what the terminal supports, once per process.
///
/// This reads the environment rather than querying the terminal. A query is
/// the protocol's blessed method, but it means writing to the tty and blocking
/// on a reply, and a terminal that answers neither the graphics query nor the
/// device-attributes fallback hangs the process — which is exactly the bug
/// this module replaces. Every terminal that implements the protocol
/// identifies itself in the environment, so the tradeoff is one-sided.
///
/// `NOFETCH_KITTY` overrides: `0`/`off` disables, `1`/`on` forces graphics,
/// `full` forces graphics and animation.
pub fn detect() -> Caps {
    static CAPS: OnceLock<Caps> = OnceLock::new();
    *CAPS.get_or_init(probe)
}

fn probe() -> Caps {
    classify(
        |key| std::env::var(key).ok().filter(|v| !v.is_empty()),
        std::io::stdout().is_terminal(),
    )
}

/// The capability table, split out from the environment so it can be tested
/// without a terminal and without mutating global state.
fn classify(env: impl Fn(&str) -> Option<String>, is_tty: bool) -> Caps {
    let forced = env("NOFETCH_KITTY").map(|v| v.to_ascii_lowercase());

    if let Some(force) = &forced
        && matches!(force.as_str(), "0" | "off" | "no" | "none")
    {
        return Caps::NONE;
    }

    // `t=f` hands the terminal a path to open, and over SSH that path names a
    // file on the wrong machine. The pixels have to travel inline instead.
    let shares_filesystem = env("SSH_CLIENT").is_none() && env("SSH_TTY").is_none();
    let graphics = Caps {
        graphics: true,
        shares_filesystem,
        animation: false,
    };

    // An explicit override still respects SSH: it says which protocol features
    // the terminal has, not where its filesystem is.
    if let Some(force) = &forced {
        return Caps {
            animation: force == "full",
            ..graphics
        };
    }

    // Escape codes aimed at a pipe are just noise in the captured output.
    if !is_tty {
        return Caps::NONE;
    }

    // Multiplexers swallow APC sequences unless passthrough is configured, and
    // an eaten image is worse than a fallback that draws. Let viuer have it.
    let term = env("TERM").unwrap_or_default();
    if env("TMUX").is_some() || term.starts_with("screen") || term.starts_with("tmux") {
        return Caps::NONE;
    }

    let term_program = env("TERM_PROGRAM").unwrap_or_default();

    if env("KITTY_WINDOW_ID").is_some() || term.contains("kitty") {
        // The only terminal that implements `a=f` frames.
        Caps {
            animation: true,
            ..graphics
        }
    } else if env("KONSOLE_VERSION").is_some() {
        // Konsole speaks the protocol but rejects file-backed transfers.
        Caps {
            shares_filesystem: false,
            ..graphics
        }
    } else if term_program.eq_ignore_ascii_case("ghostty")
        || term.contains("ghostty")
        || env("GHOSTTY_RESOURCES_DIR").is_some()
        || term_program.eq_ignore_ascii_case("WezTerm")
        || env("WEZTERM_EXECUTABLE").is_some()
        || term_program.eq_ignore_ascii_case("WarpTerminal")
    {
        // Draw kitty graphics, ignore kitty animation.
        graphics
    } else {
        Caps::NONE
    }
}

/// Pixel dimensions of one terminal cell.
struct Cell {
    w: u32,
    h: u32,
}

fn cell_size() -> &'static Cell {
    static CELL: OnceLock<Cell> = OnceLock::new();
    CELL.get_or_init(probe_cell_size)
}

fn probe_cell_size() -> Cell {
    // Only unix fills the pixel fields in, and even there it may report zeros.
    if let Ok(ws) = crossterm::terminal::window_size()
        && ws.width > 0
        && ws.height > 0
        && ws.columns > 0
        && ws.rows > 0
    {
        return Cell {
            w: u32::from(ws.width / ws.columns),
            h: u32::from(ws.height / ws.rows),
        };
    }

    // Terminals that won't say are overwhelmingly near a 1:2 cell.
    Cell { w: 10, h: 20 }
}

/// How many rows a `cols`-wide placement of a `w`x`h` image will cover.
///
/// The terminal scales the image into whatever `c`x`r` rectangle we name, so
/// naming a rectangle of the wrong shape stretches the logo. Deriving rows
/// from the real cell aspect keeps it honest, and gives the caller the row
/// count it needs to lay the info column out beside it.
pub fn rows_for(cols: u32, dimensions: (u32, u32)) -> u32 {
    rows_in_cell(cols, dimensions, cell_size())
}

/// Shrink an image to the size it will actually be drawn at.
///
/// The terminal scales whatever it is given into the `cols`x`rows` rectangle,
/// so sending full-resolution pixels puts bytes on the wire only to have them
/// thrown away — and an animation pays that cost once per frame.
fn fit_to_placement(img: DynamicImage, cols: u32, rows: u32) -> DynamicImage {
    let cell = cell_size();
    let (max_w, max_h) = (cols * cell.w, rows * cell.h);

    if img.width() <= max_w && img.height() <= max_h {
        return img;
    }

    img.resize(max_w, max_h, image::imageops::FilterType::CatmullRom)
}

fn rows_in_cell(cols: u32, (w, h): (u32, u32), cell: &Cell) -> u32 {
    if w == 0 || h == 0 || cell.h == 0 {
        return 1;
    }

    let px_w = u64::from(cols) * u64::from(cell.w);
    let px_h = px_w * u64::from(h) / u64::from(w);
    let rows = px_h.div_ceil(u64::from(cell.h));

    u32::try_from(rows).unwrap_or(u32::MAX).max(1)
}

/// A stable, non-zero id for this image.
///
/// Keyed on the path so repeated runs of the same logo reuse one entry in the
/// terminal's image store instead of pushing the store towards its quota.
fn image_id(path: &Path) -> u32 {
    let mut hasher = DefaultHasher::new();
    path.hash(&mut hasher);
    (hasher.finish() as u32).max(1)
}

/// Write one graphics command, splitting the payload across `m=` chunks.
fn command(out: &mut impl Write, control: &str, payload: &[u8]) -> std::io::Result<()> {
    if payload.is_empty() {
        return write!(out, "\x1b_G{control};\x1b\\");
    }

    let encoded = STANDARD.encode(payload);
    let mut chunks = encoded.as_bytes().chunks(CHUNK).peekable();
    let mut first = true;

    while let Some(chunk) = chunks.next() {
        let more = u8::from(chunks.peek().is_some());
        // base64 is ASCII, so the chunk is always valid UTF-8.
        let body = String::from_utf8_lossy(chunk);

        if first {
            write!(out, "\x1b_G{control},m={more};{body}\x1b\\")?;
            first = false;
        } else {
            write!(out, "\x1b_Gm={more};{body}\x1b\\")?;
        }
    }

    Ok(())
}

fn encode_png(img: &DynamicImage) -> Result<Vec<u8>, String> {
    let mut buf = std::io::Cursor::new(Vec::new());
    img.write_to(&mut buf, image::ImageFormat::Png)
        .map_err(|e| e.to_string())?;
    Ok(buf.into_inner())
}

/// Draw `path` as a `cols`-wide logo indented by `margin`, and leave the
/// cursor at the top-left of the block so the caller can print beside it.
/// Returns the number of rows the image covers.
///
/// With `animate` off a GIF takes the still path: `image::open` stops after
/// frame one, so no frames are uploaded and no `Player` comes back.
///
/// Every command carries `q=2`, which tells the terminal to stay quiet about
/// both successes and failures. Without it the replies arrive on our stdin and
/// the shell reads them as typed input once nofetch exits.
pub fn place_beside(
    path: &Path,
    cols: u32,
    margin: u32,
    animate: bool,
) -> Result<Placement, String> {
    let caps = detect();
    if !caps.graphics {
        return Err("terminal does not support the kitty graphics protocol".into());
    }

    let dimensions = image::image_dimensions(path).map_err(|e| e.to_string())?;
    let rows = rows_for(cols, dimensions);
    let id = image_id(path);

    let is_gif = path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("gif"));

    // Build the whole stream before touching stdout. Decoding and encoding can
    // both fail, and the caller answers a failure by falling back to viuer —
    // which it cannot do cleanly if we have already reserved rows and moved
    // the cursor on screen.
    let mut commands = Vec::new();
    let player = if is_gif && animate {
        draw_gif(&mut commands, path, id, cols, rows, margin, caps)?
    } else {
        draw_still(&mut commands, path, id, cols, rows, caps)?;
        None
    };

    let stdout = std::io::stdout();
    let mut out = stdout.lock();

    // Scroll the block into existence before placing anything. The terminal
    // clips an image that runs past the last row, and near the bottom of the
    // screen that is most of the logo.
    write!(out, "{}", "\n".repeat(rows as usize)).map_err(|e| e.to_string())?;
    write!(out, "\x1b[{rows}A\r").map_err(|e| e.to_string())?;
    if margin > 0 {
        write!(out, "\x1b[{margin}C").map_err(|e| e.to_string())?;
    }

    out.write_all(&commands).map_err(|e| e.to_string())?;

    // `C=1` means the cursor never moved, so it is still at the top-left of
    // the block. No save/restore pair, and nothing to go wrong if the screen
    // scrolled underneath us — the terminal scrolls placements with the text.
    out.flush().map_err(|e| e.to_string())?;
    Ok(Placement { rows, player })
}

/// The placement id every frame reuses, so a flip can delete exactly the
/// placement it is replacing instead of stacking a new one on top.
const PLACEMENT: u32 = 1;

/// `a=T` — transmit and display in one command. `C=1` keeps the cursor put.
fn placement_control(id: u32, cols: u32, rows: u32) -> String {
    format!("a=T,i={id},p={PLACEMENT},q=2,C=1,f=100,c={cols},r={rows}")
}

fn draw_still(
    out: &mut impl Write,
    path: &Path,
    id: u32,
    cols: u32,
    rows: u32,
    caps: Caps,
) -> Result<(), String> {
    let control = placement_control(id, cols, rows);

    // A PNG the terminal can open itself needs no payload at all: hand over
    // the path and no pixel data crosses the tty.
    if caps.shares_filesystem
        && path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("png"))
        && let Ok(absolute) = std::fs::canonicalize(path)
        && let Some(as_str) = absolute.to_str()
    {
        return command(out, &format!("{control},t=f"), as_str.as_bytes())
            .map_err(|e| e.to_string());
    }

    // Otherwise re-encode to PNG and send it inline. PNG rather than raw RGBA
    // because it is already deflated — roughly an order of magnitude fewer
    // bytes through the terminal for flat-coloured logos.
    let img = fit_to_placement(image::open(path).map_err(|e| e.to_string())?, cols, rows);
    let png = encode_png(&img)?;

    command(out, &format!("{control},t=d"), &png).map_err(|e| e.to_string())
}

#[allow(clippy::too_many_arguments)]
fn draw_gif(
    out: &mut impl Write,
    path: &Path,
    id: u32,
    cols: u32,
    rows: u32,
    margin: u32,
    caps: Caps,
) -> Result<Option<Player>, String> {
    let file = BufReader::new(File::open(path).map_err(|e| e.to_string())?);
    let mut frames = GifDecoder::new(file)
        .map_err(|e| e.to_string())?
        .into_frames();

    // Streamed rather than collected, so only one decoded frame is resident
    // at a time. Collecting a 12-frame 498x498 GIF would hold ~12 MB of RGBA
    // for the sake of a logo drawn 30 columns wide.
    let first = frames
        .next()
        .ok_or("gif has no frames")?
        .map_err(|e| e.to_string())?;

    let gap_ms = |frame: &image::Frame| {
        let ms = std::time::Duration::from(frame.delay()).as_millis();
        i32::try_from(ms).unwrap_or(i32::MAX).max(MIN_GAP_MS)
    };

    // The root frame is the image itself, placed the same way a still is.
    let base = fit_to_placement(DynamicImage::ImageRgba8(first.buffer().clone()), cols, rows);
    let control = placement_control(id, cols, rows);
    command(out, &format!("{control},t=d"), &encode_png(&base)?).map_err(|e| e.to_string())?;

    // Every frame gets uploaded either way. What differs is who flips them.
    let mut ids = vec![id];
    let mut gaps = vec![Duration::from_millis(gap_ms(&first) as u64)];

    for (offset, frame) in frames.enumerate() {
        let frame = frame.map_err(|e| e.to_string())?;
        let z = gap_ms(&frame);
        let img = fit_to_placement(DynamicImage::ImageRgba8(frame.into_buffer()), cols, rows);
        let png = encode_png(&img)?;

        if caps.animation {
            // `a=f` appends a frame to the image the terminal is already
            // showing. The decoder hands us fully composited buffers, so each
            // one is a complete overwrite (`X=1`) rather than a delta.
            command(out, &format!("a=f,i={id},q=2,f=100,t=d,X=1,z={z}"), &png)
                .map_err(|e| e.to_string())?;
        } else {
            // No `a=f` here, so each frame becomes an image in its own right,
            // transmitted (`a=t`) but not displayed. Playback then flips
            // between them with placement commands alone.
            let frame_id = frame_id(id, offset + 1);
            command(out, &format!("a=t,i={frame_id},q=2,f=100,t=d"), &png)
                .map_err(|e| e.to_string())?;
            ids.push(frame_id);
        }

        gaps.push(Duration::from_millis(z as u64));
    }

    // A single-frame GIF is a still image. Nothing to drive, nothing to loop.
    if ids.len() == 1 && gaps.len() == 1 {
        return Ok(None);
    }

    if !caps.animation {
        return Ok(Some(Player {
            ids,
            gaps,
            cols,
            rows,
            margin,
        }));
    }

    // The root frame is created without a gap, so it needs one set after the
    // fact or the animation would snap past frame one.
    let root_gap = gap_ms(&first);
    command(out, &format!("a=a,i={id},q=2,r=1,z={root_gap}"), &[]).map_err(|e| e.to_string())?;

    // `s=3` runs it, `v=1` loops forever. The terminal owns the animation from
    // here, so nofetch can exit immediately instead of sleeping through it.
    command(out, &format!("a=a,i={id},q=2,s=3,v=1"), &[]).map_err(|e| e.to_string())?;

    Ok(None)
}

/// Id for the `n`th frame of the image based at `base`.
///
/// Frames sit in the ids immediately after their image, so re-running with the
/// same art overwrites its own uploads instead of growing the image store.
fn frame_id(base: u32, n: usize) -> u32 {
    base.wrapping_add(n as u32).max(1)
}

/// The result of placing an image: how many rows it covers, and whether the
/// caller still has to drive the animation itself.
pub struct Placement {
    pub rows: u32,
    player: Option<Player>,
}

impl Placement {
    /// Play a client-driven animation, if this placement needs one. Blocks
    /// until the write fails or the user interrupts.
    ///
    /// Call this *after* drawing everything else. Terminals that implement
    /// `a=f` never get here — they were handed the frames and are looping on
    /// their own while this process exits.
    ///
    /// `lines_below` is how far the cursor has travelled past the image origin
    /// since it was placed, so playback can hop back up to it and return.
    pub fn animate(self, lines_below: u32) {
        if let Some(player) = self.player {
            player.play(lines_below);
        }
    }
}

/// Client-driven playback for terminals that draw kitty graphics but ignore
/// kitty animation.
///
/// Every frame is already uploaded and stays uploaded, so a flip costs one
/// delete and one placement — tens of bytes — rather than re-transmitting the
/// whole frame the way the old viuer loop did.
struct Player {
    /// One image id per frame, in order. Frame one is the root image.
    ids: Vec<u32>,
    gaps: Vec<Duration>,
    cols: u32,
    rows: u32,
    margin: u32,
}

impl Player {
    fn play(&self, lines_below: u32) {
        let stdout = std::io::stdout();
        // The root frame is already on screen, so the first flip replaces it.
        let mut showing = self.ids[0];

        loop {
            for (&id, &gap) in self.ids.iter().zip(&self.gaps) {
                if id != showing {
                    let mut out = stdout.lock();
                    if self.flip(&mut out, showing, id, lines_below).is_err() {
                        // stdout went away (piped into something that closed).
                        return;
                    }
                    showing = id;
                }
                std::thread::sleep(gap);
            }
        }
    }

    /// Swap the visible frame, leaving the cursor exactly where it started.
    fn flip(
        &self,
        out: &mut impl Write,
        showing: u32,
        next: u32,
        lines_below: u32,
    ) -> std::io::Result<()> {
        // Drop the outgoing placement but keep its pixels: lowercase `d=i`
        // frees the placement only, so the next loop reuses the upload.
        write!(out, "\x1b_Ga=d,d=i,i={showing},p={PLACEMENT},q=2;\x1b\\")?;

        // Hop up to the image origin. `a=p` places at the cursor.
        if lines_below > 0 {
            write!(out, "\x1b[{lines_below}A")?;
        }
        write!(out, "\r")?;
        if self.margin > 0 {
            write!(out, "\x1b[{}C", self.margin)?;
        }

        write!(
            out,
            "\x1b_Ga=p,i={next},p={PLACEMENT},q=2,C=1,c={},r={};\x1b\\",
            self.cols, self.rows
        )?;

        // And back down, so an interrupt leaves the cursor below the block
        // rather than in the middle of the info column.
        if lines_below > 0 {
            write!(out, "\x1b[{lines_below}B")?;
        }
        write!(out, "\r")?;

        // Both commands land in the same write, so the terminal never renders
        // the gap between the delete and the placement.
        out.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render(control: &str, payload: &[u8]) -> String {
        let mut buf = Vec::new();
        command(&mut buf, control, payload).unwrap();
        String::from_utf8(buf).unwrap()
    }

    #[test]
    fn payloadless_command_has_no_chunk_marker() {
        assert_eq!(
            render("a=a,i=7,q=2,s=3,v=1", &[]),
            "\x1b_Ga=a,i=7,q=2,s=3,v=1;\x1b\\"
        );
    }

    #[test]
    fn short_payload_is_a_single_final_chunk() {
        assert_eq!(render("a=T,i=1", b"abc"), "\x1b_Ga=T,i=1,m=0;YWJj\x1b\\");
    }

    #[test]
    fn long_payload_splits_and_repeats_only_the_marker() {
        // 4096 base64 bytes carry 3072 input bytes, so this needs two chunks.
        let payload = vec![b'x'; 3072 + 3];
        let out = render("a=T,i=1", &payload);

        assert!(out.starts_with("\x1b_Ga=T,i=1,m=1;"));
        assert!(out.ends_with("\x1b\\"));
        assert_eq!(out.matches("\x1b_G").count(), 2);
        // Control keys appear once; the continuation carries only `m`.
        assert_eq!(out.matches("a=T").count(), 1);
        assert!(out.contains("\x1b_Gm=0;"));
    }

    /// Build a `classify` environment from a list of key/value pairs.
    fn caps_in(vars: &[(&str, &str)], is_tty: bool) -> Caps {
        classify(
            |key| {
                vars.iter()
                    .find(|(k, _)| *k == key)
                    .map(|(_, v)| (*v).to_string())
            },
            is_tty,
        )
    }

    #[test]
    fn kitty_is_the_only_terminal_offered_animation() {
        assert!(caps_in(&[("TERM", "xterm-kitty")], true).animation);
        assert!(caps_in(&[("KITTY_WINDOW_ID", "1")], true).animation);

        // These draw kitty graphics but ignore `a=f`, so they must be told to
        // send a still frame rather than a stream the terminal discards.
        for still in [
            [("TERM", "xterm-ghostty")],
            [("TERM_PROGRAM", "WezTerm")],
            [("KONSOLE_VERSION", "250801")],
        ] {
            let caps = caps_in(&still, true);
            assert!(caps.graphics, "{still:?} should draw");
            assert!(!caps.animation, "{still:?} must not be sent frames");
        }
    }

    #[test]
    fn unknown_terminals_and_pipes_get_nothing() {
        assert_eq!(caps_in(&[("TERM", "xterm-256color")], true), Caps::NONE);
        assert_eq!(caps_in(&[("TERM", "xterm-kitty")], false), Caps::NONE);
    }

    #[test]
    fn multiplexers_are_left_to_the_fallback() {
        // kitty inside tmux still has TERM=xterm-kitty exported.
        assert_eq!(
            caps_in(
                &[("TERM", "xterm-kitty"), ("TMUX", "/tmp/tmux-1000/default")],
                true
            ),
            Caps::NONE
        );
        assert_eq!(caps_in(&[("TERM", "screen.xterm-kitty")], true), Caps::NONE);
    }

    #[test]
    fn ssh_forces_pixels_inline() {
        let local = caps_in(&[("TERM", "xterm-kitty")], true);
        assert!(
            local.shares_filesystem,
            "a local kitty can open the file itself"
        );

        let remote = caps_in(&[("TERM", "xterm-kitty"), ("SSH_TTY", "/dev/pts/3")], true);
        assert!(remote.graphics);
        assert!(
            !remote.shares_filesystem,
            "a `t=f` path over SSH names a file on the wrong machine"
        );
    }

    #[test]
    fn the_override_forces_support_without_lying_about_ssh() {
        // Forcing support has to work from a pipe, or it could never be tested.
        assert!(caps_in(&[("NOFETCH_KITTY", "1")], false).graphics);
        assert!(!caps_in(&[("NOFETCH_KITTY", "1")], false).animation);
        assert!(caps_in(&[("NOFETCH_KITTY", "full")], false).animation);
        assert_eq!(
            caps_in(&[("NOFETCH_KITTY", "off"), ("TERM", "xterm-kitty")], true),
            Caps::NONE
        );

        // But it must not claim a shared filesystem that isn't there.
        let forced_remote = caps_in(
            &[("NOFETCH_KITTY", "full"), ("SSH_TTY", "/dev/pts/3")],
            false,
        );
        assert!(forced_remote.animation);
        assert!(!forced_remote.shares_filesystem);
    }

    #[test]
    fn image_ids_are_never_zero() {
        assert!(image_id(Path::new("/some/art.png")) > 0);
        assert_eq!(
            image_id(Path::new("/some/art.png")),
            image_id(Path::new("/some/art.png")),
            "the same logo must reuse one slot in the image store"
        );
    }

    #[test]
    fn rows_track_the_image_aspect() {
        let cell = Cell { w: 10, h: 20 };

        // A square image over 30 columns is 300px wide and 300px tall, which
        // against a 10x20 cell is 15 rows.
        assert_eq!(rows_in_cell(30, (100, 100), &cell), 15);
        // Half as tall, half the rows.
        assert_eq!(rows_in_cell(30, (100, 50), &cell), 8);
        // Degenerate input still occupies a row rather than dividing by zero.
        assert_eq!(rows_in_cell(30, (0, 0), &cell), 1);
    }

    #[test]
    fn narrow_cells_need_more_rows_for_the_same_logo() {
        let wide = Cell { w: 10, h: 20 };
        let tall = Cell { w: 8, h: 20 };

        // Same square logo, same column count: a narrower cell means a
        // narrower image, so it needs fewer rows. Guessing one aspect for
        // every terminal is what stretches logos.
        assert_eq!(rows_in_cell(30, (100, 100), &wide), 15);
        assert_eq!(rows_in_cell(30, (100, 100), &tall), 12);
    }
}
