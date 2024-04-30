use std::{path::PathBuf};

use anyhow::bail;
use rand::seq::IteratorRandom;

use crate::{image_supplier::SavedImage, BASEDIRECTORIES};

pub struct Collection {
    name: String,
    path: PathBuf,
}

impl Collection {
    fn is_collection_name_allowed(name: &str) -> bool {
        if name.contains(' ') {
            return false;
        }

        if name.to_lowercase() != name {
            return false;
        }

        true
    }

    fn find_collection_directory(name: &str) -> anyhow::Result<Option<PathBuf>> {
        let data_dir = BASEDIRECTORIES.get_data_home();
        let directory = std::fs::read_dir(data_dir)?.find(|dir| {
            dir.as_ref().is_ok_and(|dir| {
                dir.file_type().is_ok_and(|file_type| file_type.is_dir())
                    && dir.file_name().eq_ignore_ascii_case(&name)
            })
        });

        // We can unwrap here, because the find function only searches for actual directories and ignores errors.
        Ok(directory.map(|directory| directory.unwrap().path()))
    }

    pub fn create(name: &str) -> anyhow::Result<Self> {
        if !Self::is_collection_name_allowed(name) {
            bail!("Collection name not allowed");
        }

        let collection_dir = Self::find_collection_directory(name)?;

        match collection_dir {
            Some(_) => bail!("Collection already exists"),
            None => {
                let directory_path = BASEDIRECTORIES.get_data_home().join(&name);
                std::fs::create_dir(&directory_path)?;

                Ok(Self {
                    name: name.to_owned(),
                    path: directory_path,
                })
            }
        }
    }

    pub fn open(name: &str) -> anyhow::Result<Self> {
        if !Self::is_collection_name_allowed(name) {
            bail!("No collection of name: {}", name);
        }

        let collection_dir = Self::find_collection_directory(name)?;

        match collection_dir {
            Some(directory) => Ok(Self {
                name: name.to_owned(),
                path: directory,
            }),
            None => bail!("No collection of name: {}", name),
        }
    }

    // Image management
    fn find_image_in_collection(&self, name: &str) -> anyhow::Result<Option<PathBuf>> {
        let image = std::fs::read_dir(&self.path)?.find(|file| {
            file.as_ref().is_ok_and(|file| {
                file.file_type().is_ok_and(|file_type| file_type.is_file())
                    && file.file_name().eq_ignore_ascii_case(&name)
            })
        });

        Ok(match image {
            Some(Ok(file)) => Some(file.path()),
            Some(Err(_)) => unreachable!(),
            None => None,
        })
    }

    pub fn add_image_to_collection(&self, image: &SavedImage) -> anyhow::Result<()> {
        let file_name_os = image
            .get_path()
            .file_name()
            .ok_or(anyhow::anyhow!("Image not a file"))?
            .to_ascii_lowercase();

        let file_name = file_name_os
            .to_str()
            .ok_or(anyhow::anyhow!("Image name invalid"))?;

        if self
            .find_image_in_collection(&file_name)
            .is_ok_and(|v| v.is_some())
        {
            bail!("Image already added to collection.");
        }

        let store_path = self.path.join(&file_name);

        image.copy_to(&store_path)?;

        Ok(())
    }

    pub fn get_random_image(&self) -> anyhow::Result<SavedImage> {
        let image_file = std::fs::read_dir(&self.path)?.choose(&mut rand::thread_rng());

        if let Some(image_file) = image_file {
            let path = image_file?.path();
            Ok(SavedImage::from_path(&path)?)
        } else {
            bail!("No images in collection");
        }
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }
}
