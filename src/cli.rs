use std::{path::PathBuf, process::ExitCode};

use anyhow::{anyhow, bail};
use clap::{Args, Parser, Subcommand};

use crate::{
    config::{CategoryConfig, GlobalConfig},
    image_supplier::{ImageSupplier, SearchParameters, UrlSupplier},
};

#[derive(Parser)]
#[command(name = "Background setter command line interface")]
struct Cli {
    #[command(subcommand)]
    commands: Commands,
}

#[derive(Clone, clap::Subcommand)]
enum Commands {
    /// Fetch an image from any source
    Fetch(FetchArgs),
}

#[derive(Args, Clone, Debug)]
struct FetchArgs {
    #[arg(short, long)]
    /// Whether to set the wallpaper using the 'set_command' config command.
    set: bool,
    #[arg(short, long)]
    /// Whether to put the image, goes into cache if not set.
    output: Option<PathBuf>,
    #[arg(short, long)]
    /// Which predefined category name to use.
    category: Option<String>,
    #[arg(short, long)]
    // Additional tags to add.
    tags: Vec<String>,
    #[arg(long)]
    /// Whether or not to allow non-sfw content.
    nsfw: bool,
}

impl FetchArgs {
    async fn run(self) -> anyhow::Result<()> {
        let config = GlobalConfig::read()?;
        let category = {
            match self.category {
                Some(category_name) => {
                    if config.categories.len() == 0 {
                        bail!("No categories defined in config file.");
                    }
                    let mut categories = config
                        .categories
                        .iter()
                        .map(|category| {
                            let equality = category
                                .name
                                .chars()
                                .map(|char| char.to_ascii_lowercase())
                                .zip(category_name.chars().map(|char| char.to_ascii_lowercase()))
                                .filter(|(a, b)| a == b)
                                .count();
                            (category, equality)
                        })
                        .collect::<Vec<_>>();

                    categories
                        .sort_by(|(_, simularity1), (_, simularity2)| simularity1.cmp(simularity2));

                    // Unwrap here, seeing that there being no entry in the array is checked earlier.
                    let (best_category, simularity) = *categories.first().unwrap();

                    if simularity != category_name.len() {
                        if simularity as f32 / category_name.len() as f32 >= 0.5 {
                            bail!(
                                "No category for name: {}, did you mean: {}?",
                                category_name,
                                best_category.name
                            );
                        } else {
                            bail!("No category for name: {}", category_name);
                        }
                    }

                    Some(best_category.to_owned())
                }
                None => None,
            }
        };

        let tags: Vec<String> = {
            match category {
                Some(category) => self
                    .tags
                    .into_iter()
                    .chain(category.tags.into_iter())
                    .collect(),
                None => self.tags,
            }
        };

        let parameters = SearchParameters {
            tags,
            aspect_ratios: config.aspect_ratios.unwrap_or_default(),
        };

        // TODO: Make this take a url supplier config in the constructor using a toml format
        let url_supplier = toml::from_str::<UrlSupplier>(include_str!("../image_supplier.toml"))?;
        let supplier = ImageSupplier::new(url_supplier);
        let image = supplier.get_wallpaper_image(parameters).await?;

        if let Some(output_file) = self.output {
            image.save(&output_file)?;
        }

        Ok(())
    }
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
