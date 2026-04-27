use crate::{render::visible_width, sys::SysData};
use colored::Colorize;
use regex::Regex;
use std::sync::OnceLock;

static TOKEN_REGEX: OnceLock<Regex> = OnceLock::new();

#[derive(Debug, PartialEq)]
pub enum FetchComponent {
    Os,
    Title,
    OsAge,
    Kernel,
    Uptime,
    Memory,
    Cpu,
    Colors,
    Custom,
    Packages,
    Wm,
    Display,
    Gpu,
    GpuDriver,
    Disk,
    Shell,
}

#[derive(Debug, PartialEq, Clone)]
pub enum FetchColor {
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    Gray,
}

impl FetchColor {
    pub fn from_str_name(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "black" => Some(FetchColor::Black),
            "red" => Some(FetchColor::Red),
            "green" => Some(FetchColor::Green),
            "yellow" => Some(FetchColor::Yellow),
            "blue" => Some(FetchColor::Blue),
            "magenta" => Some(FetchColor::Magenta),
            "cyan" => Some(FetchColor::Cyan),
            "white" => Some(FetchColor::White),
            "gray" => Some(FetchColor::Gray),
            _ => None,
        }
    }

    pub fn to_ansi_code(&self) -> &'static str {
        match self {
            FetchColor::Black => "\x1b[30m",
            FetchColor::Red => "\x1b[31m",
            FetchColor::Green => "\x1b[32m",
            FetchColor::Yellow => "\x1b[33m",
            FetchColor::Blue => "\x1b[34m",
            FetchColor::Magenta => "\x1b[35m",
            FetchColor::Cyan => "\x1b[36m",
            FetchColor::White => "\x1b[37m",
            FetchColor::Gray => "\x1b[90m",
        }
    }

    pub fn apply(&self, text: &str) -> colored::ColoredString {
        match self {
            FetchColor::Black => text.black(),
            FetchColor::Red => text.red(),
            FetchColor::Green => text.green(),
            FetchColor::Yellow => text.yellow(),
            FetchColor::Blue => text.blue(),
            FetchColor::Magenta => text.magenta(),
            FetchColor::Cyan => text.cyan(),
            FetchColor::White => text.white(),
            FetchColor::Gray => text.bright_black(),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum WidthMode {
    Full,
    Fit,
}

#[derive(Debug)]
pub struct FetchModule {
    pub kind: FetchComponent,
    pub label: Option<String>,
    pub value: Option<String>,
    pub color: Option<FetchColor>,
    pub icon: Option<String>,
    pub symbol: Option<String>,
    pub format: Option<String>,
    pub width: Option<WidthMode>,
    pub fill: Option<String>,
    pub thresholds: Option<Vec<f64>>,
}

impl FetchModule {
    pub fn format_output(&self, values: &[String]) -> String {
        let output = self.format.clone().unwrap_or_else(|| "{1}".to_string());

        let (thresh_med, thresh_max) = self
            .thresholds
            .as_ref()
            .and_then(|t| {
                if t.len() >= 2 {
                    Some((t[0], t[1]))
                } else {
                    None
                }
            })
            .unwrap_or((60.0, 80.0));

        let re = TOKEN_REGEX.get_or_init(|| Regex::new(r"\{(\d+)(?::([a-zA-Z_]+))?\}").unwrap());

        let result = re.replace_all(&output, |caps: &regex::Captures| {
            let index_str = caps.get(1).unwrap().as_str();

            if let Ok(index) = index_str.parse::<usize>()
                && index > 0
                && index <= values.len()
            {
                let val = &values[index - 1];

                if let Some(color_match) = caps.get(2) {
                    let color_name = color_match.as_str().to_lowercase();

                    if color_name == "dynamic" || color_name == "auto" {
                        let numeric_str: String = val
                            .chars()
                            .filter(|c| c.is_ascii_digit() || *c == '.' || *c == '-')
                            .collect();

                        if let Ok(num) = numeric_str.parse::<f64>() {
                            if num >= thresh_max {
                                return val.red().to_string();
                            } else if num >= thresh_med {
                                return val.yellow().to_string();
                            } else {
                                return val.green().to_string();
                            }
                        }
                        return val.to_string();
                    }

                    if let Some(fetch_color) = FetchColor::from_str_name(&color_name) {
                        return fetch_color.apply(val).to_string();
                    }
                }
                return val.to_string();
            }
            caps.get(0).unwrap().as_str().to_string()
        });

        result.to_string()
    }
}

#[derive(Debug)]
pub struct FetchArt {
    pub name: Option<String>,
    pub colors: Option<Vec<FetchColor>>,
}

impl FetchArt {
    pub fn get_palette(&self, os_name: &str) -> Vec<FetchColor> {
        if let Some(custom_colors) = &self.colors
            && !custom_colors.is_empty()
        {
            return custom_colors.clone();
        }

        let os = os_name.to_lowercase();

        if os.contains("cachy") {
            vec![FetchColor::Cyan, FetchColor::Green]
        } else if os.contains("arch") || os.contains("artix") {
            vec![FetchColor::Cyan, FetchColor::Blue]
        } else if os.contains("ubuntu") {
            vec![FetchColor::Red, FetchColor::Yellow, FetchColor::White]
        } else if os.contains("debian") {
            vec![FetchColor::Red, FetchColor::White]
        } else if os.contains("fedora") {
            vec![FetchColor::Blue, FetchColor::White]
        } else if os.contains("nixos") {
            vec![FetchColor::Blue, FetchColor::Cyan]
        } else if os.contains("mint") {
            vec![FetchColor::Green, FetchColor::White]
        } else if os.contains("void") {
            vec![FetchColor::Green, FetchColor::Black]
        } else if os.contains("gentoo") {
            vec![FetchColor::Magenta, FetchColor::White]
        } else if os.contains("windows") {
            vec![FetchColor::Blue, FetchColor::Cyan, FetchColor::White]
        } else if os.contains("mac") || os.contains("darwin") {
            vec![
                FetchColor::Green,
                FetchColor::Yellow,
                FetchColor::Red,
                FetchColor::Magenta,
                FetchColor::Blue,
                FetchColor::Cyan,
            ]
        } else {
            vec![FetchColor::White]
        }
    }
}

#[derive(Debug)]
pub struct FetchSchema {
    pub art: Option<FetchArt>,
    pub modules: Vec<FetchModule>,
}

impl FetchSchema {
    pub fn generate(&self, data: &SysData) -> Vec<String> {
        let mut pre_render: Vec<(Option<&FetchModule>, String)> = Vec::new();
        let mut max_width = 0;

        for module in &self.modules {
            if module.kind == FetchComponent::Colors {
                let sym_type = module.symbol.as_deref().unwrap_or("square");
                let (c, spacer) = match sym_type {
                    "circle" => ("●", " "),
                    "square" => ("███", ""),
                    "small_square" => ("▪", " "),
                    custom => (custom, " "),
                };

                let color_blocks = [
                    c.black(),
                    c.red(),
                    c.green(),
                    c.yellow(),
                    c.blue(),
                    c.magenta(),
                    c.cyan(),
                    c.white(),
                ];

                let text = format!(
                    "  {}",
                    color_blocks
                        .iter()
                        .map(|col| col.to_string())
                        .collect::<Vec<_>>()
                        .join(spacer)
                );
                max_width = max_width.max(visible_width(&text));
                pre_render.push((None, text));
                continue;
            }

            if module.kind == FetchComponent::Custom && module.fill.is_some() {
                pre_render.push((Some(module), String::new()));
                continue;
            }

            let default_label = match module.kind {
                FetchComponent::Os => "OS",
                FetchComponent::Kernel => "Kernel",
                FetchComponent::Uptime => "Uptime",
                FetchComponent::Memory => "Memory",
                FetchComponent::Cpu => "CPU",
                FetchComponent::Packages => "Packages",
                FetchComponent::Wm => "WM",
                FetchComponent::Display => "Display",
                FetchComponent::Gpu => "GPU",
                FetchComponent::OsAge => "OS Age",
                FetchComponent::GpuDriver => "GPU Driver",
                FetchComponent::Shell => "Shell",
                _ => "",
            };

            let display_label = module.label.as_deref().unwrap_or(default_label);
            let icon = module.icon.as_deref().unwrap_or(" ");
            let label_color = module.color.as_ref().unwrap_or(&FetchColor::Blue);

            let base_str = if display_label.is_empty() {
                format!("{} ", icon)
            } else {
                format!("{} {}:", icon, display_label)
            };
            let colored_label = label_color.apply(&base_str).bold();

            if module.kind == FetchComponent::Custom {
                let val = module.value.as_deref().unwrap_or("");
                let text = if display_label.is_empty() && icon.trim().is_empty() {
                    label_color.apply(val).to_string()
                } else {
                    format!("  {} {}", colored_label, val)
                };
                max_width = max_width.max(visible_width(&text));
                pre_render.push((None, text));
                continue;
            }

            let mut generate_stat_line = |stats: Vec<String>| {
                let text = format!("  {} {}", colored_label, module.format_output(&stats));
                max_width = max_width.max(visible_width(&text));
                pre_render.push((None, text));
            };

            match module.kind {
                FetchComponent::Title => {
                    generate_stat_line(vec![data.user.clone(), data.host.clone()])
                }
                FetchComponent::Os => {
                    generate_stat_line(vec![data.os.clone(), data.kernel.clone()])
                }
                FetchComponent::Display => {
                    for monitor in &data.displays {
                        generate_stat_line(vec![monitor.clone()]);
                    }
                }
                FetchComponent::Memory => {
                    let used_gib = data.mem_used_b as f64 / (1024.0 * 1024.0 * 1024.0);
                    let total_gib = data.mem_total_b as f64 / (1024.0 * 1024.0 * 1024.0);
                    let perc = if data.mem_total_b > 0 {
                        (data.mem_used_b as f64 / data.mem_total_b as f64) * 100.0
                    } else {
                        0.0
                    };

                    let stats = vec![
                        format!("{:.2} GiB", used_gib),
                        format!("{:.2} GiB", total_gib),
                        format!("{:.0}%", perc),
                    ];

                    let final_str = if module.format.is_some() {
                        module.format_output(&stats)
                    } else {
                        format!("{} / {} ({})", stats[0], stats[1], stats[2])
                    };

                    let text = format!("  {} {}", colored_label, final_str);
                    max_width = max_width.max(visible_width(&text));
                    pre_render.push((None, text));
                }
                FetchComponent::Disk => {
                    let used_gib = data.disk_used_b as f64 / (1024.0 * 1024.0 * 1024.0);
                    let total_gib = data.disk_total_b as f64 / (1024.0 * 1024.0 * 1024.0);
                    let perc = if data.disk_total_b > 0 {
                        (data.disk_used_b as f64 / data.disk_total_b as f64) * 100.0
                    } else {
                        0.0
                    };

                    let stats = vec![
                        format!("{:.2} GiB", used_gib),
                        format!("{:.2} GiB", total_gib),
                        format!("{:.0}%", perc),
                    ];

                    let final_str = if module.format.is_some() {
                        module.format_output(&stats)
                    } else {
                        format!("{} / {} ({})", stats[0], stats[1], stats[2])
                    };

                    let text = format!("  {} {}", colored_label, final_str);
                    max_width = max_width.max(visible_width(&text));
                    pre_render.push((None, text));
                }
                _ => {
                    let stat = match module.kind {
                        FetchComponent::Kernel => &data.kernel,
                        FetchComponent::Shell => &data.shell,
                        FetchComponent::Uptime => &data.uptime,
                        FetchComponent::Cpu => &data.cpu,
                        FetchComponent::Gpu => &data.gpu,
                        FetchComponent::GpuDriver => &data.gpu_driver,
                        FetchComponent::Packages => &data.packages,
                        FetchComponent::Wm => &data.wm,
                        FetchComponent::OsAge => &data.os_age,
                        _ => "",
                    };
                    generate_stat_line(vec![stat.to_string()]);
                }
            }
        }

        let mut info_lines = Vec::new();

        for (layout_module, pre_rendered_text) in pre_render {
            if let Some(module) = layout_module {
                let color = module.color.as_ref().unwrap_or(&FetchColor::White);
                let mut val = module.value.clone().unwrap_or_default();
                let fill_sym = module.fill.as_ref().unwrap();

                let target_width = match module.width.as_ref().unwrap_or(&WidthMode::Full) {
                    WidthMode::Full => max_width,
                    WidthMode::Fit => visible_width(&val.replace("{}", "")).max(10),
                };

                let caps_width = visible_width(&val.replace("{}", ""));
                let fill_count = target_width.saturating_sub(caps_width);
                let filler = fill_sym.repeat(fill_count);

                val = val.replace("{}", &filler);
                info_lines.push(color.apply(&val).to_string());
            } else {
                info_lines.push(pre_rendered_text);
            }
        }

        info_lines
    }
}
