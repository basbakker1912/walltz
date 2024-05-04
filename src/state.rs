use std::{io, path::PathBuf, str::Split};

use crate::image::ImageError;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{collections::Collection, image::SavedImage, BASEDIRECTORIES, CONFIG};

#[derive(Debug, Error)]
pub enum StateError {
    #[error("An internal file system error occured: {0}")]
    FsError(io::Error),
    #[error("An internal image error occured: {0:?}")]
    ImageError(ImageError),
    #[error("The command for assigning a wallpaper failed: {0:?}")]
    AssignCommandError(String),
    #[error("No image has been set")]
    NoImageSet,
}

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

    pub fn apply(&self, image: &SavedImage) -> Result<(), StateError> {
        let image_path = match image.get_absolute_path_as_string() {
            Ok(image_path) => image_path,
            Err(err) => return Err(StateError::ImageError(err)),
        };
        let used_args =
            self.args
                .clone()
                .into_iter()
                .map(|v| if v == "{path}" { &image_path } else { v });
        let result = std::process::Command::new(self.program)
            .args(used_args)
            .output();

        match result {
            Ok(output) => {
                if output.status.success() {
                    Ok(())
                } else {
                    Err(StateError::AssignCommandError(
                        String::from_utf8_lossy(&output.stderr).into_owned(),
                    ))
                }
            }
            Err(err) => Err(StateError::FsError(err)),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum ImageStateType {
    Image { path: String },
    Collection { name: String, image_path: String },
}

impl ImageStateType {
    pub fn get_image_path(&self) -> &str {
        match self {
            Self::Image { path } => path,
            Self::Collection {
                name: _,
                image_path,
            } => image_path,
        }
    }

    pub fn load_saved_image(&self) -> Result<SavedImage, StateError> {
        let image_path = self.get_image_path();

        match SavedImage::from_path(image_path) {
            Ok(image) => Ok(image),
            Err(err) => Err(StateError::ImageError(err)),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct State {
    image: Option<ImageStateType>,
}

impl State {
    pub fn open() -> Result<Self, StateError> {
        let file_content = std::fs::read_to_string(STATE_FILE.clone());
        match file_content {
            Ok(file_content) => Ok(toml::from_str(&file_content).unwrap_or_default()),
            // If file not found (OS ERROR 2)
            Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(Self::default()),
            Err(err) => Err(StateError::FsError(err)),
        }
    }

    pub fn assign_current_image(&self) -> Result<(), StateError> {
        if let Some(current_image) = &self.image {
            if let Some(set_command) = &CONFIG.set_command {
                let image = current_image.load_saved_image()?;
                let command = SetImageCommand::new(set_command);
                command.apply(&image)?;

                Ok(())
            } else {
                Err(StateError::AssignCommandError(
                    "Not assign command specified".to_string(),
                ))
            }
        } else {
            Ok(())
        }
    }

    pub fn get_state(&self) -> Option<&ImageStateType> {
        self.image.as_ref()
    }

    pub fn get_current_image(&self) -> Result<SavedImage, StateError> {
        if let Some(current_image) = &self.image {
            current_image.load_saved_image()
        } else {
            Err(StateError::NoImageSet)
        }
    }

    /// Sets the current image and assigns it, if there is a command specified, else it just sets the state.
    pub fn set_current_image(&mut self, image: &SavedImage) -> Result<(), StateError> {
        match image.get_absolute_path_as_string() {
            Ok(path) => {
                self.image = Some(ImageStateType::Image { path });

                Ok(())
            }
            Err(err) => return Err(StateError::ImageError(err)),
        }
    }

    pub fn set_current_collection(
        &mut self,
        collection: &Collection,
        image: &SavedImage,
    ) -> Result<(), StateError> {
        match image.get_absolute_path_as_string() {
            Ok(image_path) => {
                self.image = Some(ImageStateType::Collection {
                    name: collection.get_name().to_owned(),
                    image_path,
                });

                Ok(())
            }
            Err(err) => return Err(StateError::ImageError(err)),
        }
    }
}

impl Drop for State {
    fn drop(&mut self) {
        let file_content = toml::to_string(&self).expect("Failed to serialize state to TOML");
        std::fs::write(STATE_FILE.clone(), file_content).expect("Failed to write state to file.");
    }
}
