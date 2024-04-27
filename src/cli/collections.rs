mod create;
mod delete;
mod save;
mod use_collection;

#[derive(Clone, clap::Subcommand)]
pub enum CollectionCommands {
    Create(create::CreateCollectionArgs),
    Delete,
    Save(save::SaveImageArgs),
    Use(use_collection::UseArgs),
}

impl CollectionCommands {
    pub async fn run(self) -> anyhow::Result<()> {
        match self {
            CollectionCommands::Create(args) => args.run().await,
            CollectionCommands::Save(args) => args.run().await,
            CollectionCommands::Use(args) => args.run().await,
            _ => todo!(),
        }
    }
}
