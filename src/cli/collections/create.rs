use crate::collections::Collection;

#[derive(clap::Args, Clone, Debug)]
pub struct CreateCollectionArgs {
    name: String,
}

impl CreateCollectionArgs {
    pub async fn run(self) -> anyhow::Result<()> {
        Collection::create(&self.name)?;

        println!("Successfully created the collection: {}", self.name);

        Ok(())
    }
}
