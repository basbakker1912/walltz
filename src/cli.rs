use std::{fs, io::Read, path::PathBuf, process::ExitCode, time::Duration, vec};

use anyhow::{anyhow, bail};
use clap::{Args, Parser, Subcommand};
use indicatif::{ProgressBar, ProgressStyle};
use rand::seq::SliceRandom;
use tracing_subscriber::fmt::format;
use xdg::BaseDirectories;

use crate::{
    config::{CategoryConfig, GlobalConfig},
    image_supplier::{ImageSupplier, SearchParameters, UrlSupplier},
};

mod fetch;

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
}

pub struct Program;

impl Program {
    pub async fn init() -> ExitCode {
        let cli = Cli::parse();

        let result = match cli.commands {
            Commands::Fetch(args) => args.run().await,
        };

        match result {
            Ok(_) => ExitCode::SUCCESS,
            Err(err) => {
                println!("{}", err);
                ExitCode::FAILURE
            }
        }
    }
}
