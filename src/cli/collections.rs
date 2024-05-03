mod create;
mod delete;
mod from;
mod save;
mod sync;

#[derive(Clone, clap::Subcommand)]
pub enum CollectionCommands {
    Create(create::CreateCollectionArgs),
    Delete(delete::DeleteArgs),
    Save(save::SaveImageArgs),
    // From Remote Command
    From(from::FromArgs),
    Sync(sync::SyncArgs),
}

impl CollectionCommands {
    pub async fn run(self) -> anyhow::Result<()> {
        match self {
            CollectionCommands::Create(args) => args.run().await,
            CollectionCommands::Delete(args) => args.run().await,
            CollectionCommands::Save(args) => args.run().await,
            CollectionCommands::From(args) => args.run().await,
            CollectionCommands::Sync(args) => args.run().await,
        }
    }
}
