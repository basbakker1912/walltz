use clap::Args;

use crate::collections::Collection;

#[derive(Clone, Args)]
pub struct ListArgs {}

impl ListArgs {
    pub fn run(self) -> anyhow::Result<()> {
        let collections = Collection::list()?;

        for collection in collections {
            println!("{}", collection);
        }

        Ok(())
    }
}
