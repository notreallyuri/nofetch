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
    #[arg(short, long)]
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

    let data = sys::gather_info();

    let art_name = {
        if args.art != "auto" && !args.art.trim().is_empty() {
            args.art.clone()
        } else if let Some(art_config) = &config.art
            && let Some(fixed_name) = &art_config.name
            && !fixed_name.trim().is_empty()
        {
            fixed_name.clone()
        } else {
            data.os
                .to_lowercase()
                .replace(" linux", "")
                .replace(" ", "")
        }
    };

    let info_lines = config.generate(&data);

    if let Some(path) = config::get_art_path(&art_name) {
        if config::is_image(&path) {
            render::draw_with_image(&path, &info_lines);
        } else {
            let raw_logo = config::load_ascii(&art_name).unwrap_or_else(|_| {
                fail_fast("Art", &art_name, "arts");
            });

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

            let colored_logo = config::colorize_ascii(raw_logo, &palette);
            render::draw(&colored_logo, &info_lines);
        }
    } else {
        fail_fast("Art", &art_name, "arts");
    }

    if args.performance {
        let duration = start_time.elapsed();
        println!("\n  {} Finished in: {:?}", "󱫐".yellow(), duration);
    }
}

fn fail_fast(kind: &str, name: &str, folder: &str) -> ! {
    eprintln!("  {} {} '{}' not found.", "󰅙".red(), kind, name.bold());

    if folder.is_empty() {
        eprintln!(
            "  {} Expected config at ~/.config/nothings/nofetch.lua",
            "󰌵".blue()
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
                "  {} No files found in ~/.config/nothings/{}",
                "󰌵".blue(),
                folder
            );
        }
    }

    std::process::exit(1);
}
