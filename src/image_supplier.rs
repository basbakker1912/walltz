use std::{
    fs,
    io::{self, Cursor},
    path::{Path, PathBuf},
    str::FromStr,
};

use bytes::Bytes;
use image::ImageFormat;

mod url_supplier;

use thiserror::Error;
pub use url_supplier::UrlSupplier;

use crate::BASEDIRECTORIES;

pub struct SearchParameters {
    pub tags: Vec<String>,
    pub aspect_ratios: Vec<String>,
}

#[derive(Error, Debug)]
pub enum ImageError {
    #[error("The image doesn't exist")]
    NotFound,
    #[error("The format of the image is not supported, or not an image")]
    InvalidFormat,
    #[error("Cannot convert the image to the specified format")]
    IncompatibleFormat,
    #[error("An internal file system error occured: {0}")]
    FsError(io::Error),
    #[error("Failed to write image to file")]
    WriteFailed,
    #[error("Failed to fetch image from url: {0:?}")]
    FetchError(reqwest::Error),
}

pub struct SavedImage {
    path: PathBuf,
    format: ImageFormat,
}

impl SavedImage {
    pub fn from_path<P>(path: P) -> Result<Self, ImageError>
    where
        P: AsRef<Path>,
    {
        let path = path.as_ref();

        if !path.is_file() {
            return Err(ImageError::NotFound);
        }

        let format = match ImageFormat::from_path(path) {
            Ok(format) => format,
            Err(_) => return Err(ImageError::InvalidFormat),
        };

        Ok(Self {
            path: path.to_owned(),
            format,
        })
    }

    pub fn get_absolute_path(&self) -> Result<PathBuf, ImageError> {
        match fs::canonicalize(&self.path) {
            Ok(pathbuf) => Ok(pathbuf),
            Err(err) => Err(ImageError::FsError(err)),
        }
    }

    pub fn get_absolute_path_as_string(&self) -> Result<String, ImageError> {
        let image_path_buf = self.get_absolute_path()?;
        let image_path = image_path_buf.to_string_lossy();

        Ok(image_path.into_owned())
    }

    pub fn get_path(&self) -> &Path {
        &self.path
    }

    pub fn get_name(&self) -> Result<String, ImageError> {
        let file_stem = match self.path.file_stem() {
            Some(file_stem) => file_stem,
            None => return Err(ImageError::NotFound),
        };
        let file_name = file_stem.to_string_lossy();

        Ok(file_name.into_owned())
    }

    pub fn get_format(&self) -> ImageFormat {
        self.format
    }

    pub fn copy_to<P>(&self, path: P) -> Result<SavedImage, ImageError>
    where
        P: AsRef<Path>,
    {
        let path = path.as_ref();

        match ImageFormat::from_path(&path) {
            Ok(format) if format == self.format => {
                fn inner(from_path: &Path, goal_path: &Path) -> Result<(), io::Error> {
                    let image_data = std::fs::read(from_path)?;
                    std::fs::write(goal_path, image_data)?;

                    Ok(())
                }

                inner(&self.path, path).map_err(|err| ImageError::FsError(err))?;
                Ok(SavedImage::from_path(path)?)
            }
            Ok(_) => Err(ImageError::IncompatibleFormat),
            Err(_) => Err(ImageError::InvalidFormat),
        }
    }
}

pub struct FetchedImage {
    id: String,
    bytes: Bytes,
    format: ImageFormat,
}

impl FetchedImage {
    /// Unlike save, this encodes the image correctly. SLOW
    pub fn save_to_format<P>(&self, path: P) -> Result<SavedImage, ImageError>
    where
        P: AsRef<Path>,
    {
        let path = path.as_ref();

        if self
            .format
            .extensions_str()
            .first()
            .is_some_and(|ext| path.ends_with(ext))
        {
            self.save(path)?;

            return Ok(SavedImage {
                path: path.to_owned(),
                format: self.format,
            });
        }

        let reader = image::io::Reader::with_format(Cursor::new(self.bytes.clone()), self.format);

        match reader.decode().and_then(|image| image.save(path)) {
            Ok(_) => Ok(SavedImage {
                path: path.to_owned(),
                format: self.format,
            }),
            Err(_err) => Err(ImageError::WriteFailed),
        }
    }

    /// Just saves the file in it's pred
    pub fn save(&self, path: &Path) -> Result<SavedImage, ImageError> {
        let image_data = self.bytes.as_ref();
        match std::fs::write(path, image_data) {
            Ok(_) => Ok(SavedImage {
                path: path.to_owned(),
                format: self.format,
            }),
            Err(err) => Err(ImageError::FsError(err)),
        }
    }

    pub fn cache(&self) -> Result<SavedImage, ImageError> {
        let cache_dir = BASEDIRECTORIES.get_cache_home();

        match std::fs::create_dir_all(&cache_dir) {
            Ok(_) => {
                let file_name = {
                    // Unwrap here because there will always be atleast 1 extension for a format
                    let extension = self.format.extensions_str().first().unwrap();

                    format!("wallpaper_{}.{}", self.id, extension)
                };
                let image_path = cache_dir.join(file_name);
                let saved_image = self.save(&image_path)?;

                Ok(saved_image)
            }
            Err(err) => Err(ImageError::FsError(err)),
        }
    }

    pub async fn fetch_from_url(image_url: ImageUrlObject) -> Result<Self, ImageError> {
        async fn fetch_bytes(url: &str) -> Result<Bytes, reqwest::Error> {
            let image_result = reqwest::get(url).await?;
            let image_bytes = image_result.bytes().await?;

            Ok(image_bytes)
        }

        match fetch_bytes(&image_url.url).await {
            Ok(bytes) => Ok(FetchedImage {
                id: image_url.id,
                bytes,
                format: image_url.image_format,
            }),
            Err(err) => Err(ImageError::FetchError(err)),
        }
    }
}

#[derive(Debug)]
pub struct ImageUrlObject {
    id: String,
    url: String,
    image_format: ImageFormat,
}

impl ImageUrlObject {
    pub fn from_url(url: String) -> Result<Self, ImageError> {
        // Invalible error
        let url_path = std::path::PathBuf::from_str(&url).unwrap();
        let id = url_path
            .file_stem()
            .map_or(uuid::Uuid::new_v4().to_string(), |v| {
                v.to_os_string()
                    .into_string()
                    .unwrap_or(uuid::Uuid::new_v4().to_string())
            });

        match ImageFormat::from_path(&url_path) {
            Ok(image_format) => Ok(Self {
                id,
                url: url,
                image_format,
            }),
            Err(_err) => Err(ImageError::InvalidFormat),
        }
    }
}

pub struct ExternalImage<'a> {
    path: &'a str,
}

impl<'a> ExternalImage<'a> {
    async fn fetch_from_url(&self) -> Result<SavedImage, ImageError> {
        let image_url = ImageUrlObject::from_url(self.path.to_owned())?;
        let fetched_image = FetchedImage::fetch_from_url(image_url).await?;
        fetched_image.cache()
    }

    pub fn new(path: &'a str) -> Self {
        Self { path }
    }

    pub async fn load(&self) -> Result<SavedImage, ImageError> {
        match self.path {
            url if url.starts_with("https://") || url.starts_with("http://") => {
                self.fetch_from_url().await
            }
            path if PathBuf::from_str(&path).is_ok_and(|v| v.is_file()) => {
                let result = SavedImage::from_path(&path);

                match result {
                    Ok(image) => Ok(image),
                    Err(_err) => Err(ImageError::NotFound),
                }
            }
            _ => Err(ImageError::NotFound),
        }
    }
}
