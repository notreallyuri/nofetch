use std::time::Instant;

use clap::Parser;
use colored::Colorize;

pub mod config;
pub mod render;
pub mod schema;
pub mod sys;

#[derive(Parser)]
struct Args {
    #[arg(short, long, default_value = "nofetch")]
    config: String,
    #[arg(short, long, default_value = "auto")]
    art: String,
    #[arg(short = 'C', long)]
    color: Option<String>,
    #[arg(short, long, action = clap::ArgAction::SetTrue)]
    performance: bool,
}

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
            render::draw_with_image(&path, &info_lines);
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
