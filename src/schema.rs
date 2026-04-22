use crate::sys::SysData;
use colored::Colorize;
use serde::Deserialize;

#[derive(Deserialize, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum FetchComponent {
    Os,
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
    #[serde(rename = "gpu_driver")]
    GpuDriver,
}

#[derive(Deserialize, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
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

#[derive(Deserialize, Debug)]
pub struct FetchModule {
    #[serde(rename = "type")]
    pub kind: FetchComponent,
    pub label: Option<String>,
    pub value: Option<String>,
    pub color: Option<FetchColor>,
    pub icon: Option<String>,
    pub symbol: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct FetchArt {
    pub color: Option<FetchColor>,
}

#[derive(Deserialize, Debug)]
pub struct FetchSchema {
    pub art: Option<FetchArt>,
    pub modules: Vec<FetchModule>,
}

impl FetchSchema {
    pub fn generate(&self, data: &SysData) -> Vec<String> {
        let mut info_lines = Vec::new();

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
                    c.white(),
                    c.cyan(),
                    c.magenta(),
                    c.blue(),
                    c.yellow(),
                    c.green(),
                    c.red(),
                ];

                let color_str = color_blocks
                    .iter()
                    .map(|col| col.to_string())
                    .collect::<Vec<_>>()
                    .join(spacer);

                info_lines.push(format!("  {}", color_str));
                continue;
            }

            if module.kind == FetchComponent::Custom
                && module.label.is_none()
                && module.icon.is_none()
            {
                let val = module.value.as_deref().unwrap_or("");
                let color = module.color.as_ref().unwrap_or(&FetchColor::White);
                info_lines.push(color.apply(val).to_string());
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
                FetchComponent::GpuDriver => "GPU Driver",
                FetchComponent::Custom => "",
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

            match module.kind {
                FetchComponent::Os => info_lines.push(format!("  {} {}", colored_label, data.os)),
                FetchComponent::Kernel => {
                    info_lines.push(format!("  {} {}", colored_label, data.kernel))
                }
                FetchComponent::Uptime => {
                    info_lines.push(format!("  {} {}", colored_label, data.uptime))
                }
                FetchComponent::Memory => {
                    info_lines.push(format!("  {} {}", colored_label, data.mem_str))
                }
                FetchComponent::Cpu => info_lines.push(format!("  {} {}", colored_label, data.cpu)),
                FetchComponent::Gpu => info_lines.push(format!("  {} {}", colored_label, data.gpu)), // ADD THIS
                FetchComponent::GpuDriver => {
                    info_lines.push(format!("  {} {}", colored_label, data.gpu_driver))
                }
                FetchComponent::Packages => {
                    info_lines.push(format!("  {} {}", colored_label, data.packages))
                }
                FetchComponent::Wm => info_lines.push(format!("  {} {}", colored_label, data.wm)),
                FetchComponent::Display => {
                    for monitor in &data.displays {
                        info_lines.push(format!("  {} {}", colored_label, monitor));
                    }
                }
                FetchComponent::Custom => {
                    let val = module.value.as_deref().unwrap_or("");
                    info_lines.push(format!("  {} {}", colored_label, val));
                }
                FetchComponent::Colors => {}
            }
        }

        info_lines
    }
}
