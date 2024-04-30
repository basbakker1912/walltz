use anyhow::bail;
use clap::Args;

use crate::{
    collections::Collection,
    image_supplier::{ExternalImage, SavedImage},
    state::State,
};

enum FetchImageResultData {
    Image(SavedImage),
    Collection(Collection, SavedImage),
}

#[derive(Args, Debug, Clone)]
pub struct SetArgs {
    /// Which parameter to set to, default is a path
    name: String,
}

impl SetArgs {
    async fn fetch_image(&self) -> anyhow::Result<FetchImageResultData> {
        let external_image = ExternalImage::new(&self.name).load().await;

        if let Ok(image) = external_image {
            return Ok(FetchImageResultData::Image(image));
        }

        let collection = Collection::open(&self.name);

        if let Ok(collection) = collection {
            let image = collection.get_random_image()?;

            return Ok(FetchImageResultData::Collection(collection, image));
        }

        // TODO: Fetch from category here

        bail!("Invalid name");
    }

    pub async fn run(self) -> anyhow::Result<()> {
        let image = self.fetch_image().await?;

        let mut state = State::load()?;
        let image_path = match image {
            FetchImageResultData::Image(image) => {
                state.set_current_image(&image)?;
                image.get_absolute_path()?
            }
            FetchImageResultData::Collection(collection, image) => {
                state.set_current_collection(&collection, &image)?;
                image.get_absolute_path()?
            }
        };
        state.assign_current_image()?;

        println!("Set wallpaper to image: {:?}", image_path);

        Ok(())
    }
}
