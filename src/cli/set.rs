use std::{path::PathBuf, str::FromStr};

use anyhow::bail;
use clap::Args;
use reqwest::Url;

use crate::{
    collections::Collection,
    image::{ExternalImage, FetchedImage, ImageUrl, SavedImage},
    state::{ImageStateType, State},
};

enum FetchImageResultData {
    Image(SavedImage),
    Collection(Collection, SavedImage),
}

#[derive(Args, Debug, Clone)]
pub struct SetArgs {
    #[arg(short, long)]
    /// Reapply the last wallpaper set.
    reapply: bool,
    /// Which name to search for.
    name: Option<String>,
}

impl SetArgs {
    async fn fetch_image(name: &str) -> anyhow::Result<FetchImageResultData> {
        let external_image = ExternalImage::new(name.to_owned()).load().await;

        if let Ok(image) = external_image {
            return Ok(FetchImageResultData::Image(image));
        }

        let collection = Collection::open(&name);

        if let Ok(collection) = collection {
            let image = collection.get_directory().get_random_image()?;

            return Ok(FetchImageResultData::Collection(collection, image));
        }

        // TODO: Fetch from category here

        bail!("The name: {} is not a valid url, path or collection", name);
    }

    pub async fn run(self) -> anyhow::Result<()> {
        let mut state = State::open()?;

        if self.reapply {
            let image_state = state.get_state();
            if let Some(image_state) = image_state {
                match image_state {
                    ImageStateType::Image { path } => {
                        if !PathBuf::from_str(&path).is_ok_and(|v| v.is_file()) {
                            bail!("Cannot reapply, targeted file no longer exists");
                        }
                        state.assign_current_image()?;
                    }
                    ImageStateType::Collection {
                        name,
                        image_path: _,
                    } => {
                        let colletion = Collection::open(&name)?;
                        state.set_current_collection(
                            &colletion,
                            &colletion.get_directory().get_random_image()?,
                        )?;
                        state.assign_current_image()?;
                    }
                }
            } else {
                bail!("No state set to reapply");
            }
            return Ok(());
        }

        if let Some(name) = self.name {
            let image = Self::fetch_image(&name).await?;

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
        } else {
            bail!("Please specify a valid argument.");
        }

        Ok(())
    }
}
