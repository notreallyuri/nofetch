use std::str::FromStr;

use super::{color::FetchColor, kind::StatKind};

#[derive(Debug, PartialEq)]
pub enum WidthMode {
    Full,
    Fit,
}

#[derive(Debug)]
pub struct StatModule {
    pub kind: StatKind,
    pub label: Option<String>,
    pub icon: Option<String>,
    pub color: Option<FetchColor>,
    pub format: Option<String>,
    pub separator: Option<String>,
    pub thresholds: Option<[f64; 2]>,
}

#[derive(Debug)]
pub struct SeparatorModule {
    pub fill: String,
    pub value: Option<String>,
    pub width: WidthMode,
    pub color: Option<FetchColor>,
}

#[derive(Debug)]
pub struct TextModule {
    pub value: String,
    pub color: Option<FetchColor>,
}

#[derive(Debug)]
pub struct ColorsModule {
    pub symbol: ColorSymbol,
}

#[derive(Debug)]
pub enum ColorSymbol {
    Circle,
    Square,
    SmallSquare,
    Custom(String),
}

#[derive(Debug)]
pub enum Module {
    Stat(StatModule),
    Separator(SeparatorModule),
    Text(TextModule),
    Colors(ColorsModule),
}

impl FromStr for ColorSymbol {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "circle" => ColorSymbol::Circle,
            "small_square" => ColorSymbol::SmallSquare,
            "square" => ColorSymbol::Square,
            other => ColorSymbol::Custom(other.to_string()),
        })
    }
}

impl ColorSymbol {
    pub fn as_parts(&self) -> (&str, &str) {
        match self {
            ColorSymbol::Circle => ("●", " "),
            ColorSymbol::Square => ("███", ""),
            ColorSymbol::SmallSquare => ("▪", " "),
            ColorSymbol::Custom(s) => (s, " "),
        }
    }
}
