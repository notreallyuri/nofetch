use clap::Parser;
use colored::Colorize;

pub mod config;
pub mod render;
pub mod schema;
pub mod sys;

#[derive(Parser)]
struct Args {
    #[arg(short, long, default_value = "default")]
    schema: String,
    #[arg(short, long, default_value = "auto")]
    art: String,
    #[arg(short, long)]
    color: Option<String>,
}

fn main() {
    let args = Args::parse();

    // 1. Load Schema (Fail fast if invalid config)
    let schema = config::load_schema(&args.schema).unwrap_or_else(|e| {
        eprintln!("  {} Schema error: {}", "󰅙".red(), e.dimmed());
        fail_fast("Schema", &args.schema, "schemas");
    });

    // 2. Gather data FIRST.
    // This gives us `data.os` to use for both logo fallback and color logic.
    let data = sys::gather_info();

    // 3. Determine the ASCII art name
    let art_name = if args.art == "auto" {
        // E.g., "CachyOS Linux" -> "cachyos"
        data.os
            .to_lowercase()
            .replace(" linux", "")
            .replace(" ", "")
    } else {
        args.art.clone()
    };

    let raw_logo = config::load_ascii(&art_name).unwrap_or_else(|_| {
        fail_fast("Art", &art_name, "ascii/logos");
    });

    let info_lines = schema.generate(&data);

    let palette = if let Some(cli_color_str) = &args.color {
        if let Some(c) = schema::FetchColor::from_str_name(cli_color_str) {
            vec![c]
        } else {
            vec![schema::FetchColor::White]
        }
    } else {
        // Delegate to the smart engine: JSON custom array -> Native OS fallback
        if let Some(art_config) = &schema.art {
            art_config.get_palette(&data.os)
        } else {
            schema::FetchArt { colors: None }.get_palette(&data.os)
        }
    };

    let colored_logo = config::colorize_ascii(raw_logo, &palette);

    render::draw(&colored_logo, &info_lines);
}

fn fail_fast(kind: &str, name: &str, folder: &str) -> ! {
    eprintln!("  {} {} '{}' not found.", "󰅙".red(), kind, name.bold());

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

    std::process::exit(1);
}
