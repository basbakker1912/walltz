use std::{path::PathBuf, time::Duration};

use image::ImageFormat;

use crate::BASEDIRECTORIES;

use super::{FetchedImage, ImageError, SavedImage};

/// A image cache manager, does cleanup next to saving and retrieving images.
pub struct ImageCache {
    path: PathBuf,
}

impl ImageCache {
    pub fn cleanup_cache(&self) -> Result<(), ImageError> {
        let files = match self.path.read_dir() {
            Ok(files) => files,
            Err(err) => return Err(ImageError::FsError(err)),
        };

        fn should_cull(dir: std::fs::DirEntry) -> Option<PathBuf> {
            let dir_path = dir.path();

            let is_image = ImageFormat::from_path(&dir_path).is_ok();
            if !is_image {
                return None;
            }

            let last_modified = match dir.metadata().and_then(|metadata| metadata.modified()) {
                Ok(modified) => modified,
                Err(_) => return None,
            };

            let elapsed_since_modified = match last_modified.elapsed() {
                Ok(elapsed) => elapsed,
                Err(_) => return None,
            };

            // 7 days before deletion
            // TODO: Make this a value in the config
            const DELETION_THRESHOLD: Duration = Duration::from_secs(7 * 24 * 60 * 60);

            if elapsed_since_modified > DELETION_THRESHOLD {
                Some(dir_path)
            } else {
                None
            }
        }

        for file_path in files.filter_map(|dir| dir.ok().and_then(should_cull)) {
            match std::fs::remove_file(file_path) {
                Ok(_) => {}
                Err(err) => return Err(ImageError::FsError(err)),
            }
        }

        Ok(())
    }

    pub fn open() -> Self {
        let this = Self {
            path: BASEDIRECTORIES.get_cache_home(),
        };

        this
    }

    pub fn find(&self, name: &str) -> Result<SavedImage, ImageError> {
        let mut files = match self.path.read_dir() {
            Ok(direntries) => direntries,
            Err(err) => return Err(ImageError::FsError(err)),
        };

        let file = files.find_map(|direntry| match direntry {
            Ok(direntry) => {
                let file_path = direntry.path();
                let stem = match file_path.file_stem() {
                    Some(stem) => stem.to_string_lossy(),
                    None => return None,
                };

                if stem == name {
                    Some(file_path)
                } else {
                    None
                }
            }
            Err(_) => None,
        });

        match file {
            Some(file_path) => SavedImage::from_path(file_path),
            None => Err(ImageError::NotFound),
        }
    }

    pub fn cache(&self, image: &FetchedImage) -> Result<SavedImage, ImageError> {
        let file_name = image.get_file_name();
        match BASEDIRECTORIES.place_cache_file(file_name) {
            Ok(file_path) => image.save(&file_path),
            Err(err) => Err(ImageError::FsError(err)),
        }
    }
}
