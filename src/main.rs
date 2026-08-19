use std::time::Instant;

use clap::Parser;
use colored::Colorize;

pub mod config;
pub mod render;
pub mod schema;
pub mod sys;

#[derive(Parser)]
#[command(
    version,
    about = "A system fetch: Lua-configured modules drawn beside a terminal logo.",
    after_long_help = LONG_HELP
)]
struct Args {
    /// Config to load, by name
    ///
    /// Resolved as `<config dir>/<NAME>.lua`. A missing config is an error,
    /// and nofetch lists the ones it did find.
    #[arg(short, long, value_name = "NAME", default_value = "nofetch")]
    config: String,

    /// Logo to draw, by name, or `auto` to guess from the OS
    ///
    /// Resolved under `<config dir>/arts/`: `.txt` draws as ASCII, `.png`,
    /// `.jpg` and `.gif` draw as images. Overrides `art.name` in the config.
    #[arg(short, long, value_name = "NAME", default_value = "auto")]
    art: String,

    /// Paint ASCII art in a single color, overriding the config
    ///
    /// Only affects ASCII logos — image art carries its own colors.
    #[arg(
        short = 'C',
        long,
        value_name = "COLOR",
        ignore_case = true,
        value_parser = clap::builder::PossibleValuesParser::new(schema::color::FetchColor::NAMES)
    )]
    color: Option<String>,

    /// Draw animated art as a single still frame
    ///
    /// Only changes anything where nofetch would otherwise have to drive the
    /// frames itself and block until interrupted. See ANIMATION below.
    #[arg(short = 'A', long, alias = "no-anim")]
    no_animation: bool,

    /// Print how long the run took
    #[arg(short, long)]
    performance: bool,
}

const LONG_HELP: &str = "\
FILES:
  Config and art live in ~/.config/nothings, or %APPDATA%\\nothings on Windows:
    nofetch.lua          the config --config names, alongside any others
    arts/<name>.txt      ASCII logos, also read from arts/logos/
    arts/<name>.gif      image logos: .png, .jpg and .gif draw as pixels

  With --art auto (the default) the logo is guessed from the OS, and a built-in
  one is drawn if nothing matches — so a fresh install still renders. A logo
  asked for by name has to exist.

ANIMATION:
  GIF art is handed to the terminal to animate wherever the protocol allows it,
  and nofetch exits immediately:
    kitty                                     kitty graphics frames
    WezTerm, iTerm2, mintty, rio, Warp,
    Konsole                                   iTerm2 protocol

  Everywhere else — Ghostty included — nothing native exists, so nofetch flips
  the frames itself and blocks until Ctrl-C. The info column is already on
  screen by then, but the prompt does not come back. Pass --no-animation (-A)
  to draw the first frame and exit.

ENVIRONMENT:
  NOFETCH_KITTY=0|1|full   override kitty graphics detection: off, graphics, or
                           graphics plus terminal-driven animation
";

fn main() {
    let start_time = Instant::now();
    let args = Args::parse();

    let config = config::load_config(&args.config).unwrap_or_else(|e| {
        eprintln!("  {} Schema error: {}", "󰅙".red(), e.dimmed());
        fail_fast("Schema", &args.config, "schemas");
    });

    let data = sys::gather_info(&config.modules);

    // Whether the logo was asked for by name (CLI flag or `art.name`) or merely
    // guessed from the OS. A missing named logo is an error; a missing guess
    // falls back to the built-in one so a fresh install still renders.
    let (art_name, art_explicit) = {
        if args.art != "auto" && !args.art.trim().is_empty() {
            (args.art.clone(), true)
        } else if let Some(art_config) = &config.art
            && let Some(fixed_name) = &art_config.name
            && !fixed_name.trim().is_empty()
        {
            (fixed_name.clone(), true)
        } else if data.os.to_lowercase() == "darwin" {
            ("macos".to_string(), false)
        } else {
            (
                data.os
                    .to_lowercase()
                    .replace(" linux", "")
                    .replace(" ", ""),
                false,
            )
        }
    };

    let info_lines = config.generate(&data);

    let palette = if let Some(cli_color_str) = &args.color {
        schema::color::FetchColor::from_str_name(cli_color_str)
            .map(|c| vec![c])
            .unwrap_or(vec![schema::color::FetchColor::White])
    } else {
        config.art.as_ref().map_or_else(
            || {
                schema::Art {
                    name: None,
                    colors: None,
                }
                .get_palette(&data.os)
            },
            |art_config| art_config.get_palette(&data.os),
        )
    };

    match config::get_art_path(&art_name) {
        Some(path) if config::is_image(&path) => {
            render::draw_with_image(&path, &info_lines, !args.no_animation);
        }
        Some(_) => {
            let raw_logo = config::load_ascii(&art_name).unwrap_or_else(|_| {
                fail_fast("Art", &art_name, "arts");
            });
            render::draw(&config::colorize_ascii(raw_logo, &palette), &info_lines);
        }
        None if art_explicit => fail_fast("Art", &art_name, "arts"),
        None => {
            let fallback = config::colorize_ascii(config::default_ascii(), &palette);
            render::draw(&fallback, &info_lines);
        }
    }

    if args.performance {
        let duration = start_time.elapsed();
        println!("\n  {} Finished in: {:?}", "󱫐".yellow(), duration);
    }
}

fn fail_fast(kind: &str, name: &str, folder: &str) -> ! {
    eprintln!("  {} {} '{}' not found.", "󰅙".red(), kind, name.bold());

    let config_base = config::get_config_path();

    if folder.is_empty() {
        eprintln!(
            "  {} Expected config at {}/nofetch.lua",
            "󰌵".blue(),
            config_base.display()
        );
    } else {
        let available = config::list_available_configs(folder);
        if !available.is_empty() {
            eprintln!(
                "  {} Available {}: {}",
                "󰌵".blue(),
                folder,
                available.join(", ").cyan()
            );
        } else {
            eprintln!(
                "  {} No files found in {}",
                "󰌵".blue(),
                config_base.join(folder).display()
            );
        }
    }

    std::process::exit(1);
}
