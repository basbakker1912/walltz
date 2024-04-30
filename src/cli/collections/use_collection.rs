use crate::{collections::Collection, state::State};

#[derive(clap::Args, Clone, Debug)]
pub struct UseArgs {
    /// The name of the collection you wish to use.
    collection: String,
}

impl UseArgs {
    pub async fn run(self) -> anyhow::Result<()> {
        let collection = Collection::open(&self.collection)?;

        let image = collection.get_random_image()?;

        let mut state = State::load()?;
        state.set_current_collection(&collection, &image)?;
        state.assign_current_image()?;

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
