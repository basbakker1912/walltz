use std::{collections::HashMap, io::Read};

use serde::Deserialize;
use xdg::BaseDirectories;

use crate::BASEDIRECTORIES;

#[derive(Deserialize, Clone, Debug)]
pub struct CategoryConfig {
    pub name: String,
    pub tags: Vec<String>,
}

#[derive(Deserialize, Clone, Debug, Default)]
pub struct GlobalConfig {
    pub set_command: Option<String>,
    pub aspect_ratios: Option<Vec<String>>,
    #[serde(default)]
    pub categories: Vec<CategoryConfig>,
}

impl GlobalConfig {
    pub fn read() -> anyhow::Result<Self> {
        let config_path = BASEDIRECTORIES.place_config_file("config.toml").unwrap();

        let settings = std::fs::File::open(config_path);

        match settings {
            Ok(mut file) => {
                let mut buf = String::new();
                file.read_to_string(&mut buf)?;

                Ok(toml::from_str(&buf)?)
            }
            Err(_) => Ok(GlobalConfig::default()),
        }
    }
}
