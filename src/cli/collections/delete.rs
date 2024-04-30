use clap::Args;

use crate::collections::Collection;

#[derive(Debug, Clone, Args)]
pub struct DeleteArgs {
    /// Force delete (no prompt)
    #[arg(short, long)]
    force: bool,
    /// The name of the collection to delete.
    name: String,
}

impl DeleteArgs {
    pub async fn run(self) -> anyhow::Result<()> {
        let collection = Collection::open(&self.name)?;

        if !self.force {
            println!("Are you sure you want to delete the collection: {} and all it's images? This action is not reversable", self.name);
            let mut buffer = String::new();
            std::io::stdin().read_line(&mut buffer)?;

            if !buffer.starts_with('y') {
                return Ok(());
            }
        }

        collection.delete()?;

        println!("Successfully deleted the collection");

        Ok(())
    }
}
