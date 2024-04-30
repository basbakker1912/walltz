use std::{
    path::PathBuf,
    str::{FromStr, Split},
};

use anyhow::bail;
use serde::{Deserialize, Serialize};

use crate::{collections::Collection, image_supplier::SavedImage, BASEDIRECTORIES, CONFIG};

lazy_static::lazy_static! { static ref STATE_FILE: PathBuf = BASEDIRECTORIES.place_data_file("state.toml").expect("Failed to get state file."); }

struct SetImageCommand<'a> {
    program: &'a str,
    args: Split<'a, char>,
}

impl<'a> SetImageCommand<'a> {
    pub fn new(command: &'a str) -> SetImageCommand<'a> {
        let (program, args) = command.split_once(' ').unwrap_or((command, ""));
        let args = args.split(' ');

        SetImageCommand { program, args }
    }

    pub fn apply(&self, image: &SavedImage) -> anyhow::Result<()> {
        let image_path = image.get_absolute_path_as_string()?;
        let used_args =
            self.args
                .clone()
                .into_iter()
                .map(|v| if v == "{path}" { &image_path } else { v });
        let result = std::process::Command::new(self.program)
            .args(used_args)
            .output()?;

        if result.status.success() {
            Ok(())
        } else {
            bail!(String::from_utf8(result.stderr)?)
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
enum ImageType {
    Image { path: String },
    Collection { name: String, image_path: String },
}

impl ImageType {
    pub fn get_image_path(&self) -> &str {
        match self {
            Self::Image { path } => path,
            Self::Collection {
                name: _,
                image_path,
            } => image_path,
        }
    }

    pub fn load_saved_image(&self) -> anyhow::Result<SavedImage> {
        let image_path = self.get_image_path();

        SavedImage::from_path(image_path)
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct State {
    image: Option<ImageType>,
}

impl State {
    pub fn load() -> anyhow::Result<Self> {
        let file_content = std::fs::read_to_string(STATE_FILE.clone());

        match file_content {
            Ok(file_content) => Ok(toml::from_str(&file_content).unwrap_or_default()),
            // If file not found (OS ERROR 2)
            Err(err) if err.raw_os_error().is_some_and(|v| v == 2) => Ok(Self::default()),
            Err(err) => bail!(err),
        }
    }

    pub fn assign_current_image(&self) -> anyhow::Result<()> {
        if let Some(current_image) = &self.image {
            if let Some(set_command) = &CONFIG.set_command {
                let image = current_image.load_saved_image()?;
                let command = SetImageCommand::new(set_command);
                command.apply(&image)?;
            } else {
                bail!("Failed to assign image, no set command specified in config");
            }
        }

        Ok(())
    }

    pub fn get_current_image(&self) -> anyhow::Result<SavedImage> {
        if let Some(current_image) = &self.image {
            current_image.load_saved_image()
        } else {
            bail!("No background image currently set.")
        }
    }

    /// Sets the current image and assigns it, if there is a command specified, else it just sets the state.
    pub fn set_current_image(&mut self, image: &SavedImage) -> anyhow::Result<()> {
        self.image = Some(ImageType::Image {
            path: image.get_absolute_path_as_string()?,
        });

        Ok(())
    }

    pub fn set_current_collection(
        &mut self,
        collection: &Collection,
        image: &SavedImage,
    ) -> anyhow::Result<()> {
        self.image = Some(ImageType::Collection {
            name: collection.get_name().to_owned(),
            image_path: image.get_absolute_path_as_string()?,
        });

        Ok(())
    }
}

impl Drop for State {
    fn drop(&mut self) {
        let file_content = toml::to_string(&self).expect("Failed to serialize state to TOML");
        std::fs::write(STATE_FILE.clone(), file_content).expect("Failed to write state to file.");
    }
}
