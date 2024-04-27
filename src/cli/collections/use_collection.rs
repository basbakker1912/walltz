use anyhow::bail;

use crate::{collections::Collection, config::GlobalConfig};

#[derive(clap::Args, Clone, Debug)]
pub struct UseArgs {
    /// The name of the collection you wish to use.
    collection: String,
}

impl UseArgs {
    pub async fn run(self) -> anyhow::Result<()> {
        let config = GlobalConfig::read()?;
        let set_wallpaper_command = {
            if let Some(cmd) = config.set_command {
                cmd
            } else {
                bail!("Cannot use a collection without setting a wallpaper.");
            }
        };

        let collection = Collection::open(&self.collection)?;

        let image = collection.get_random_image()?;

        image.apply_with_command(&set_wallpaper_command)?;

        println!(
            "Set wallpaper to image: {}",
            image
                .get_absolute_path()?
                .to_str()
                .ok_or(anyhow::anyhow!("Image path not valid"))?
        );

        Ok(())
    }
}
