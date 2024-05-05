mod create;
mod delete;
mod from;
mod list;
mod save;
mod sync;

#[derive(Clone, clap::Subcommand)]
pub enum CollectionCommands {
    Create(create::CreateCollectionArgs),
    Delete(delete::DeleteArgs),
    Save(save::SaveImageArgs),
    From(from::FromArgs),
    Sync(sync::SyncArgs),
    List(list::ListArgs),
}

impl CollectionCommands {
    pub fn run(self) -> anyhow::Result<()> {
        match self {
            CollectionCommands::Create(args) => args.run(),
            CollectionCommands::Delete(args) => args.run(),
            CollectionCommands::Save(args) => args.run(),
            CollectionCommands::From(args) => args.run(),
            CollectionCommands::Sync(args) => args.run(),
            CollectionCommands::List(args) => args.run(),
        }
    }
}
