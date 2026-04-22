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

    let logo = config::load_ascii(&args.art).unwrap_or_else(|_| {
        let os_raw = sysinfo::System::name().unwrap_or_else(|| "linux".to_string());

        let os_normalized = os_raw.to_lowercase().replace(" linux", "").replace(" ", "");

        config::load_ascii(&os_normalized).unwrap_or_else(|_| {
            fail_fast("Art", &args.art, "ascii/logos");
        })
    });

    let schema = config::load_schema(&args.schema).unwrap_or_else(|e| {
        eprintln!("  {} Schema error: {}", "󰅙".red(), e.dimmed());
        fail_fast("Schema", &args.schema, "schemas");
    });

    let data = sys::gather_info();
    let info_lines = schema.generate(&data);

    let cli_color = args
        .color
        .and_then(|c| schema::FetchColor::from_str_name(&c));
    let config_color = schema.art.as_ref().and_then(|a| a.color.as_ref());

    let art_color = cli_color.as_ref().or(config_color);

    render::draw(&logo, &info_lines, art_color);
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
