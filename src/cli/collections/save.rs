use std::fs;

use anyhow::bail;

use crate::{collections::Collection, config::GlobalConfig, image_supplier::SavedImage};

#[derive(clap::Args, Clone, Debug)]
pub struct SaveImageArgs {
    /// Collection
    collection: String,
    /// Can be a image path or url, or leave empty to query the current wallpaper
    which: Option<String>,
}

impl SaveImageArgs {
    pub async fn run(self) -> anyhow::Result<()> {
        match self.which {
            Some(_) => todo!(),
            None => {
                let config = GlobalConfig::read()?;

                let collection = Collection::open(&self.collection)?;

                let query_script = {
                    if let Some(script) = config.query_script {
                        script
                    } else {
                        bail!("No query script set in config");
                    }
                };
                let image = SavedImage::query_from_script(&query_script)?;
                collection.add_image_to_collection(&image)?;

                println!("Added current wallpaper to collection: {}", self.collection);
            }
        }

        Ok(())
    }
}
