mod create;
mod delete;
mod save;

#[derive(Clone, clap::Subcommand)]
pub enum CollectionCommands {
    Create(create::CreateCollectionArgs),
    Delete(delete::DeleteArgs),
    Save(save::SaveImageArgs),
}

impl CollectionCommands {
    pub async fn run(self) -> anyhow::Result<()> {
        match self {
            CollectionCommands::Create(args) => args.run().await,
            CollectionCommands::Delete(args) => args.run().await,
            CollectionCommands::Save(args) => args.run().await,
        }
    }
}
