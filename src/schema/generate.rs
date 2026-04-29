use super::{Schema, color::FetchColor, kind::StatKind, module::Module};
use crate::{render::visible_width, sys::SysData};
use colored::Colorize;
use regex::Regex;
use std::sync::OnceLock;

static TOKEN_REGEX: OnceLock<Regex> = OnceLock::new();

pub fn format_output(
    format: Option<&str>,
    thresholds: Option<[f64; 2]>,
    values: &[String],
    label: &str,
) -> String {
    let output = format.unwrap_or("{1}");
    let [thresh_med, thresh_max] = thresholds.unwrap_or([60.0, 80.0]);

    let re = TOKEN_REGEX.get_or_init(|| Regex::new(r"\{(\w+)(?::([a-zA-Z_]+))?\}").unwrap());

    re.replace_all(output, |caps: &regex::Captures| {
        let token = &caps[1];

        let val = if token == "label" {
            label.to_string()
        } else if let Ok(index) = token.parse::<usize>()
            && index > 0
            && index <= values.len()
        {
            values[index - 1].clone()
        } else {
            return caps[0].to_string();
        };

        match caps.get(2).map(|m| m.as_str().to_lowercase()).as_deref() {
            Some("dynamic") | Some("auto") => {
                let num: String = val
                    .chars()
                    .filter(|c| c.is_ascii_digit() || *c == '.' || *c == '-')
                    .collect();
                match num.parse::<f64>() {
                    Ok(n) if n >= thresh_max => val.red().to_string(),
                    Ok(n) if n >= thresh_med => val.yellow().to_string(),
                    Ok(_) => val.green().to_string(),
                    Err(_) => val,
                }
            }
            Some(color_name) => FetchColor::from_str_name(color_name)
                .map(|c| c.apply(&val).to_string())
                .unwrap_or(val),
            None => val,
        }
    })
    .to_string()
}

impl Schema {
    pub fn generate(&self, data: &SysData) -> Vec<String> {
        enum PreRendered<'a> {
            Line(String),
            Separator(&'a super::module::SeparatorModule),
        }

        let mut pre_render: Vec<PreRendered> = Vec::new();
        let mut max_width: usize = 0;

        for module in &self.modules {
            match module {
                Module::Colors(m) => {
                    let (c, spacer) = m.symbol.as_parts();
                    let blocks = [
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
                        blocks
                            .iter()
                            .map(|b| b.to_string())
                            .collect::<Vec<_>>()
                            .join(spacer)
                    );
                    max_width = max_width.max(visible_width(&text));
                    pre_render.push(PreRendered::Line(text));
                }

                Module::Separator(m) => {
                    pre_render.push(PreRendered::Separator(m));
                }

                Module::Text(m) => {
                    let text = match &m.color {
                        Some(c) => c.apply(&m.value).to_string(),
                        None => m.value.clone(),
                    };
                    max_width = max_width.max(visible_width(&text));
                    pre_render.push(PreRendered::Line(text));
                }

                Module::Stat(m) => {
                    let default_label = m.kind.default_label();
                    let display_label = m.label.as_deref().unwrap_or(default_label);
                    let icon = m.icon.as_deref().unwrap_or(" ");
                    let value_color = m.color.as_ref();

                    let label_in_format = m.format.as_deref().is_some_and(|f| f.contains("{label"));

                    let plain_label = if label_in_format || display_label.is_empty() {
                        if icon.trim().is_empty() {
                            "".bold()
                        } else {
                            format!("{} ", icon).bold()
                        }
                    } else {
                        let sep = m.separator.as_deref().unwrap_or(":");
                        format!("{} {}{} ", icon, display_label, sep).bold()
                    };

                    let mut push_line = |stats: Vec<String>, default_fmt: Option<String>| {
                        let body = if let Some(fmt) = default_fmt {
                            match value_color {
                                Some(c) => c.apply(&fmt).to_string(),
                                None => fmt,
                            }
                        } else {
                            format_output(m.format.as_deref(), m.thresholds, &stats, display_label)
                        };
                        let text = format!("  {} {}", plain_label, body);
                        max_width = max_width.max(visible_width(&text));
                        pre_render.push(PreRendered::Line(text));
                    };

                    match &m.kind {
                        StatKind::Title => {
                            push_line(vec![data.user.clone(), data.host.clone()], None);
                        }
                        StatKind::Os => {
                            push_line(vec![data.os.clone(), data.kernel.clone()], None);
                        }
                        StatKind::Display => {
                            for monitor in &data.displays {
                                push_line(vec![monitor.clone()], None);
                            }
                        }
                        StatKind::Memory => {
                            let (used, total, perc) = gib_stats(data.mem_used_b, data.mem_total_b);
                            let stats = vec![used, total, perc];
                            let default_fmt = m
                                .format
                                .is_none()
                                .then(|| format!("{} / {} ({})", stats[0], stats[1], stats[2]));
                            push_line(stats, default_fmt);
                        }
                        StatKind::Disk => {
                            let default_path = if cfg!(target_os = "windows") {
                                "C:\\"
                            } else {
                                "/"
                            };
                            let target = m.path.as_deref().unwrap_or(default_path);

                            let (used, total) = data.disks.get(target).copied().unwrap_or((0, 0));

                            let (used_str, total_str, perc_str) = gib_stats(used, total);
                            let stats = vec![used_str, total_str, perc_str];
                            let default_fmt = m
                                .format
                                .is_none()
                                .then(|| format!("{} / {} ({})", stats[0], stats[1], stats[2]));
                            push_line(stats, default_fmt);
                        }
                        kind => {
                            let stat: &str = match kind {
                                StatKind::Kernel => &data.kernel,
                                StatKind::Shell => &data.shell,
                                StatKind::Uptime => &data.uptime,
                                StatKind::Cpu => &data.cpu,
                                StatKind::Gpu => &data.gpu,
                                StatKind::GpuDriver => &data.gpu_driver,
                                StatKind::Packages => &data.packages,
                                StatKind::Wm => &data.wm,
                                StatKind::OsAge => &data.os_age,
                                _ => "",
                            };
                            push_line(vec![stat.to_string()], None);
                        }
                    }
                }
            }
        }

        pre_render
            .into_iter()
            .map(|item| match item {
                PreRendered::Line(text) => text,
                PreRendered::Separator(m) => {
                    let template = m.value.as_deref().unwrap_or("{}");
                    let color = m.color.as_ref().unwrap_or(&FetchColor::White);
                    let caps_width = visible_width(&template.replace("{}", ""));
                    let target = match m.width {
                        super::module::WidthMode::Full => max_width,
                        super::module::WidthMode::Fit => caps_width.max(10),
                    };
                    let filler = m.fill.repeat(target.saturating_sub(caps_width));
                    color.apply(&template.replace("{}", &filler)).to_string()
                }
            })
            .collect()
    }
}

fn gib_stats(used_b: u64, total_b: u64) -> (String, String, String) {
    let gib = |b: u64| b as f64 / (1024.0 * 1024.0 * 1024.0);
    let perc = if total_b > 0 {
        (used_b as f64 / total_b as f64) * 100.0
    } else {
        0.0
    };
    (
        format!("{:.2} GiB", gib(used_b)),
        format!("{:.2} GiB", gib(total_b)),
        format!("{:.0}%", perc),
    )
}
