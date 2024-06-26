use std::process::ExitCode;

use clap::Parser;

mod collections;
mod fetch;
mod get;
mod set;

#[derive(Parser)]
#[command(name = "Background setter command line interface")]
struct Cli {
    #[command(subcommand)]
    commands: Commands,
}

#[derive(Clone, clap::Subcommand)]
enum Commands {
    /// Fetch an image from any source
    Fetch(fetch::FetchArgs),
    Collections {
        #[command(subcommand)]
        commands: collections::CollectionCommands,
    },
    Get(get::GetArgs),
    /// Allows for quickly setting a wallpaper
    /// Seach order: Url, Path, Collection, Category, Tag
    #[clap(visible_alias("use"))]
    Set(set::SetArgs),
}

pub struct Program;

impl Program {
    pub fn init() -> ExitCode {
        let cli = Cli::parse();

        let result = match cli.commands {
            Commands::Fetch(args) => args.run(),
            Commands::Collections { commands } => commands.run(),
            Commands::Get(args) => args.run(),
            Commands::Set(args) => args.run(),
        };

        match result {
            Ok(_) => ExitCode::SUCCESS,
            Err(err) => {
                println!("Command failed: '{:?}'", err);
                ExitCode::FAILURE
            }
        }
    }
}
