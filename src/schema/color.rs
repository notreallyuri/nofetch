use colored::Colorize;

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
    /// Every name `from_str_name` accepts. Kept beside it so `--help` can list
    /// the palette without a second copy of the table drifting out of date.
    pub const NAMES: [&'static str; 9] = [
        "black", "red", "green", "yellow", "blue", "magenta", "cyan", "white", "gray",
    ];

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
