use std::{
    fs,
    io::Cursor,
    path::{Path, PathBuf},
    str::FromStr,
};

use anyhow::{anyhow, bail};
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

#[derive(Error, Debug, Clone)]
enum LoadImageError {
    #[error("The image at path: {0:?} doesn't exist")]
    ImageDoesntExistError(PathBuf),
}

pub struct SavedImage {
    path: PathBuf,
    format: ImageFormat,
}

impl SavedImage {
    pub fn from_path<P>(path: P) -> anyhow::Result<Self>
    where
        P: AsRef<Path>,
    {
        let path = path.as_ref();

        if !path.is_file() {
            bail!(LoadImageError::ImageDoesntExistError(path.to_owned()))
        }

        let format = ImageFormat::from_path(path)?;

        Ok(Self {
            path: path.to_owned(),
            format,
        })
    }

    pub fn get_absolute_path(&self) -> anyhow::Result<PathBuf> {
        Ok(fs::canonicalize(&self.path)?)
    }

    pub fn get_absolute_path_as_string(&self) -> anyhow::Result<String> {
        let image_path_buf = self.get_absolute_path()?;
        let image_path_str = image_path_buf
            .to_str()
            .ok_or(anyhow::anyhow!("Failed to get image path"))?;

        Ok(image_path_str.to_string())
    }

    pub fn get_path(&self) -> &Path {
        &self.path
    }

    pub fn copy_to<P>(&self, path: P) -> anyhow::Result<()>
    where
        P: AsRef<Path>,
    {
        let path = path.as_ref();

        if ImageFormat::from_path(&path)? != self.format {
            bail!("Cannot copy to another format");
            // TODO: Make a function to do this.
        }

        let image_data = std::fs::read(&self.path)?;
        std::fs::write(path, image_data)?;

        Ok(())
    }
}

pub struct FetchedImage {
    id: String,
    bytes: Bytes,
    format: ImageFormat,
}

impl FetchedImage {
    /// Unlike save, this encodes the image correctly. SLOW
    pub fn save_to_format(&self, path: &Path) -> anyhow::Result<SavedImage> {
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
        let image = reader.decode()?;

        image.save(path)?;

        Ok(SavedImage {
            path: path.to_owned(),
            format: self.format,
        })
    }

    /// Just saves the file in it's pred
    pub fn save(&self, path: &Path) -> anyhow::Result<SavedImage> {
        let image_data = self.bytes.as_ref();
        std::fs::write(path, image_data)?;

        Ok(SavedImage {
            path: path.to_owned(),
            format: self.format,
        })
    }

    pub fn cache(&self) -> anyhow::Result<SavedImage> {
        let cache_dir = BASEDIRECTORIES.get_cache_home();
        std::fs::create_dir_all(&cache_dir)?;
        let file_name = {
            let extension = self
                .format
                .extensions_str()
                .first()
                .ok_or(anyhow!("No valid file format found"))?;

            format!("wallpaper_{}.{}", self.id, extension)
        };
        let image_path = cache_dir.join(file_name);
        let saved_image = self.save(&image_path)?;

        Ok(saved_image)
    }

    pub async fn fetch_from_url(image_url: ImageUrlObject) -> anyhow::Result<Self> {
        let image_result = reqwest::get(&image_url.url).await?;
        let image_bytes = image_result.bytes().await?;

        Ok(FetchedImage {
            id: image_url.id,
            bytes: image_bytes,
            format: image_url.image_format,
        })
    }
}

#[derive(Debug)]
pub struct ImageUrlObject {
    id: String,
    url: String,
    image_format: ImageFormat,
}

impl ImageUrlObject {
    pub fn from_url<P>(url: P) -> anyhow::Result<Self>
    where
        P: AsRef<str>,
    {
        let url = url.as_ref();
        let url_path = std::path::PathBuf::from_str(url)?;
        let id = url_path
            .file_stem()
            .map_or(uuid::Uuid::new_v4().to_string(), |v| {
                v.to_os_string()
                    .into_string()
                    .unwrap_or(uuid::Uuid::new_v4().to_string())
            });

        let image_format = ImageFormat::from_path(&url_path)?;

        Ok(Self {
            id,
            url: url.to_owned(),
            image_format,
        })
    }
}

pub struct ImageSupplier {
    url_supplier: UrlSupplier,
}

impl ImageSupplier {
    pub async fn get_wallpaper_image(
        &self,
        parameters: SearchParameters,
    ) -> anyhow::Result<FetchedImage> {
        let image_url = self.url_supplier.clone().search(parameters).await?;
        let image = FetchedImage::fetch_from_url(image_url).await?;

        Ok(image)
    }

    pub fn new(url_supplier: UrlSupplier) -> Self {
        Self { url_supplier }
    }
}

#[derive(Debug, Error)]
enum ExternalImageError {
    #[error("The specified path: {0}, is invalid")]
    InvalidPathError(String),
    #[error("Failed to fetch the image, reason: {0:?}")]
    FetchImageFailedError(anyhow::Error),
    #[error("No file at path: {0}, reason: {1:?}")]
    NoFileAtPathError(String, anyhow::Error),
}

pub struct ExternalImage<'a> {
    path: &'a str,
}

impl<'a> ExternalImage<'a> {
    async fn fetch_from_url(&self) -> anyhow::Result<SavedImage> {
        let image_url = ImageUrlObject::from_url(&self.path)?;
        let fetched_image = FetchedImage::fetch_from_url(image_url).await?;
        fetched_image.cache()
    }

    pub fn new(path: &'a str) -> Self {
        Self { path }
    }

    pub async fn load(&self) -> anyhow::Result<SavedImage> {
        let image = match self.path {
            url if url.starts_with("https://") || url.starts_with("http://") => {
                let result = self.fetch_from_url().await;

                match result {
                    Ok(image) => image,
                    Err(err) => bail!(ExternalImageError::FetchImageFailedError(err)),
                }
            }
            path if PathBuf::from_str(&path).is_ok_and(|v| v.is_file()) => {
                let result = SavedImage::from_path(&path);

                match result {
                    Ok(image) => image,
                    Err(err) => bail!(ExternalImageError::NoFileAtPathError(path.to_owned(), err)),
                }
            }
            _ => bail!(ExternalImageError::InvalidPathError(self.path.to_owned())),
        };

        Ok(image)
    }
}
