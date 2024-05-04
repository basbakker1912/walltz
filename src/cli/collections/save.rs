use crate::{collections::Collection, image::ExternalImage, state::State};

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

        let state = State::open()?;

        let image = match self.which {
            Some(path) => ExternalImage::new(&path).load().await?,
            None => state.get_current_image()?,
        };

        collection.get_directory().add_image(&image)?;

        println!("Added current wallpaper to collection: {}", self.collection);

        Ok(())
    }
}
