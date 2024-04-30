use std::{fs, path::PathBuf, str::FromStr};

use anyhow::bail;

use crate::{
    collections::Collection,
    config::GlobalConfig,
    image_supplier::{ExternalImage, FetchedImage, ImageUrlObject, SavedImage},
    state::State,
};

#[derive(clap::Args, Clone, Debug)]
pub struct SaveImageArgs {
    /// Collection
    collection: String,
    /// Can be a image path or url, or leave empty to query the current wallpaper
    which: Option<String>,
}

impl SaveImageArgs {
    pub async fn run(self) -> anyhow::Result<()> {
        let collection = Collection::open(&self.collection)?;

        let state = State::load()?;

        let image = match self.which {
            Some(path) => ExternalImage::new(&path).load().await?,
            None => state.get_current_image()?,
        };

        collection.add_image_to_collection(&image)?;

        println!("Added current wallpaper to collection: {}", self.collection);

        Ok(())
    }
}
