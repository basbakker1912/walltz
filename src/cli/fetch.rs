use std::{fs, io::Read, path::PathBuf, time::Duration};

use anyhow::{anyhow, bail};
use clap::Args;
use indicatif::ProgressBar;
use rand::seq::SliceRandom;

use crate::{
    config::GlobalConfig,
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
    /// Whether or not to allow non-sfw content, false by default
    nsfw: bool,
    #[arg(long)]
    /// Only return the images final path, for use in scripts.
    simple: bool,
}

impl FetchArgs {
    pub async fn run(self) -> anyhow::Result<()> {
        let category = {
            match self.category {
                Some(category_name) => {
                    if CONFIG.categories.len() == 0 {
                        bail!("No categories defined in config file.");
                    }
                    let mut categories = CONFIG
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
                        .sort_by(|(_, simularity1), (_, simularity2)| simularity2.cmp(simularity1));

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
                            bail!(
                                "No category for name: {}, did you mean one of these:{}",
                                category_name,
                                categories
                                    .iter()
                                    .take(5)
                                    .map(|v| format!("\n- {}", v.0.name))
                                    .collect::<String>()
                            );
                        }
                    }

                    Some(best_category.to_owned())
                }
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
                    aspect_ratios: category
                        .aspect_ratios
                        .unwrap_or(CONFIG.aspect_ratios.clone()),
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
                    let mut suppliers = CONFIG
                        .suppliers
                        .iter()
                        .map(|category| {
                            let equality = category
                                .name
                                .chars()
                                .map(|char| char.to_ascii_lowercase())
                                .zip(supplier_name.chars().map(|char| char.to_ascii_lowercase()))
                                .filter(|(a, b)| a == b)
                                .count();
                            (category, equality)
                        })
                        .collect::<Vec<_>>();

                    suppliers
                        .sort_by(|(_, simularity1), (_, simularity2)| simularity1.cmp(simularity2));

                    // Unwrap here, seeing that there being no entry in the array is checked earlier.
                    let (best_supplier, simularity) = *suppliers.first().unwrap();

                    if simularity != supplier_name.len() {
                        if simularity as f32 / supplier_name.len() as f32 >= 0.5 {
                            bail!(
                                "No category for name: {}, did you mean: {}?",
                                supplier_name,
                                best_supplier.name
                            );
                        } else {
                            bail!("No suppliers for name: {}", supplier_name);
                        }
                    }

                    best_supplier
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
            let mut state = State::load()?;
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
