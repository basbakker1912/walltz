use std::{io::Read, path::PathBuf};

use serde::Deserialize;

use crate::BASEDIRECTORIES;

#[derive(Deserialize, Clone, Debug)]
pub struct CategoryConfig {
    pub name: String,
    pub tags: Vec<String>,
    pub aspect_ratios: Option<Vec<String>>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct SupplierFile {
    pub name: String,
    pub file: String,
}

#[derive(Deserialize, Clone, Debug, Default)]
pub struct GlobalConfig {
    pub set_command: Option<String>,
    pub private_key_path: Option<String>,
    #[serde(default)]
    pub categories: Vec<CategoryConfig>,
    #[serde(default)]
    pub suppliers: Vec<SupplierFile>,
    #[serde(default)]
    pub aspect_ratios: Vec<String>,
}

impl GlobalConfig {
    pub fn get_config_path() -> PathBuf {
        BASEDIRECTORIES.get_config_home()
    }

    pub fn read() -> anyhow::Result<Self> {
        let config_path = BASEDIRECTORIES.place_config_file("config.toml")?;

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
