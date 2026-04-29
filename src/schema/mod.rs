use crate::schema::module::Module;

pub mod color;
pub mod generate;
pub mod kind;
pub mod module;

use color::FetchColor;

#[derive(Debug)]
pub struct Art {
    pub name: Option<String>,
    pub colors: Option<Vec<FetchColor>>,
}

impl Art {
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
pub struct Schema {
    pub art: Option<Art>,
    pub modules: Vec<Module>,
}
