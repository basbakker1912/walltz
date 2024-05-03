use std::{fs, path::PathBuf, time::Duration};

use anyhow::{anyhow, bail};
use clap::Args;
use indicatif::ProgressBar;
use rand::seq::SliceRandom;

use crate::{
    category::Category,
    config::GlobalConfig,
    finder::{check_string_equality, find_best_by_value},
    image_supplier::{ImageSupplier, SearchParameters},
    state::State,
    CONFIG,
};

#[derive(Args, Clone, Debug)]
pub struct FetchArgs {
    #[arg(short, long)]
    /// Whether to assign the wallpaper using the 'set_command' config command.
    assign: bool,
    #[arg(short, long)]
    /// Whether to put the image, goes into cache if not set.
    output: Option<PathBuf>,
    #[arg(short, long)]
    /// Which predefined category name to use.
    category: Option<String>,
    #[arg(short, long)]
    /// Which supplier to use, leave empty to pick randomly.
    supplier: Option<String>,
    #[arg(short, long)]
    // Additional tags to add.
    tags: Vec<String>,
    #[arg(long)]
    /// Only return the images final path, for use in scripts.
    simple: bool,
}

impl FetchArgs {
    pub async fn run(self) -> anyhow::Result<()> {
        let category = {
            match self.category {
                Some(category_name) => Some(Category::find_in_config(&category_name)?),
                None => None,
            }
        };

        let parameters = {
            match category {
                Some(category) => SearchParameters {
                    tags: self
                        .tags
                        .into_iter()
                        .chain(category.tags.into_iter())
                        .collect(),
                    aspect_ratios: category.aspect_ratios,
                },
                // TODO: Add aspect ratio arg in cli
                None => SearchParameters {
                    tags: self.tags,
                    aspect_ratios: CONFIG.aspect_ratios.clone(),
                },
            }
        };

        // TODO: Move this to a function.
        let url_supplier = {
            if CONFIG.suppliers.len() == 0 {
                bail!("No suppliers defined in config file.");
            }

            let supplier_file = match self.supplier {
                Some(supplier_name) => {
                    let (equal, best_value) = find_best_by_value(
                        supplier_name.as_str(),
                        CONFIG.suppliers.iter(),
                        |value| value.name.as_str(),
                        |v1, v2| check_string_equality(v1, v2),
                    );

                    if let Some(value) = best_value {
                        if equal {
                            value
                        } else {
                            bail!(
                                "No category for name: {}, did you mean: {}?",
                                supplier_name,
                                value.name
                            );
                        }
                    } else {
                        bail!("No suppliers for name: {}", supplier_name);
                    }
                }
                None => {
                    // Unwrap here, seeing that there being no entry in the array is checked earlier.
                    CONFIG.suppliers.choose(&mut rand::thread_rng()).unwrap()
                }
            };

            let file_path = GlobalConfig::get_config_path().join(&supplier_file.file);
            let file = std::fs::read_to_string(&file_path);

            match file {
                Ok(file_content) => toml::from_str(&file_content)?,
                Err(err) => {
                    bail!(
                        "Failed to read supplier file: {:?}, reason: {} ",
                        file_path,
                        err
                    );
                }
            }
        };
        let supplier = ImageSupplier::new(url_supplier);

        let image = if self.simple {
            supplier.get_wallpaper_image(parameters).await?
        } else {
            let pb = ProgressBar::new_spinner();
            pb.enable_steady_tick(Duration::from_millis(120));
            pb.set_message("Downloading...");
            let image = supplier.get_wallpaper_image(parameters).await?;
            pb.finish_with_message("Downloaded");

            image
        };

        let saved_image = if let Some(output_file) = self.output {
            let pb = ProgressBar::new_spinner();
            pb.enable_steady_tick(Duration::from_millis(120));
            pb.set_message("Saving image to file...");
            let saved_image = image.save_to_format(&output_file)?;
            pb.finish_with_message(format!(
                "Successfully saved image to file: {}",
                fs::canonicalize(&output_file)?
                    .to_str()
                    .ok_or(anyhow!("Failed to convert image path to string."))?
            ));

            saved_image
        } else {
            image.cache()?
        };

        if self.assign {
            let mut state = State::open()?;
            state.set_current_image(&saved_image)?;
            let result = state.assign_current_image();

            match result {
                Ok(_) if !self.simple => println!("Assigned to image as the active wallpaper."),
                Ok(_) => {}
                Err(err) => {
                    println!("Failed to assign wallpaper: {}", err)
                }
            }
        }

        if self.simple {
            println!(
                "{}",
                saved_image
                    .get_absolute_path()?
                    .to_str()
                    .ok_or(anyhow!("Image file was not saved"))?
            );
        }

        Ok(())
    }
}
