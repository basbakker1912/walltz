use std::{
    fs::{self, metadata},
    io::{self, BufReader, Cursor},
    path::{Path, PathBuf},
    str::FromStr,
    time::Duration,
};

use bytes::Bytes;
use image::ImageFormat;

mod url_supplier;

use reqwest::Url;
use thiserror::Error;
pub use url_supplier::UrlSupplier;

use crate::{BASEDIRECTORIES, IMAGECACHE};

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
    #[error("The supplied url is invalid")]
    InvalidUrl,
    #[error("The supplied external image location is invalid")]
    InvalidExternal,
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

enum FetchedImageType {
    Storage(SavedImage),
    Memory(Bytes),
}

pub struct FetchedImage {
    stem: String,
    data: FetchedImageType,
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

        let result = match &self.data {
            FetchedImageType::Memory(bytes) => {
                let reader =
                    image::io::Reader::with_format(Cursor::new(bytes.clone()), self.format);

                reader.decode().and_then(|image| image.save(path))
            }
            FetchedImageType::Storage(saved) => {
                let reader = image::io::Reader::with_format(
                    io::BufReader::new(
                        fs::File::open(saved.get_path()).map_err(|err| ImageError::FsError(err))?,
                    ),
                    self.format,
                );

                reader.decode().and_then(|image| image.save(path))
            }
        };

        match result {
            Ok(_) => Ok(SavedImage {
                path: path.to_owned(),
                format: self.format,
            }),
            Err(_err) => Err(ImageError::WriteFailed),
        }
    }

    /// Just saves the file in it's pred
    pub fn save(&self, path: &Path) -> Result<SavedImage, ImageError> {
        match &self.data {
            FetchedImageType::Memory(bytes) => {
                if !path.extension().is_some_and(|ext| {
                    self.format
                        .extensions_str()
                        .contains(&ext.to_string_lossy().as_ref())
                }) {
                    return Err(ImageError::IncompatibleFormat);
                }

                let image_data = bytes.as_ref();
                match std::fs::write(path, image_data) {
                    Ok(_) => Ok(SavedImage {
                        path: path.to_owned(),
                        format: self.format,
                    }),
                    Err(err) => Err(ImageError::FsError(err)),
                }
            }
            FetchedImageType::Storage(saved) => saved.copy_to(path),
        }
    }

    pub fn get_file_extension(&self) -> &str {
        self.format.extensions_str().first().unwrap()
    }

    pub fn get_file_name(&self) -> String {
        format!("{}.{}", self.stem, self.get_file_extension())
    }

    /// Fetch the image from the url, or grab it out of cache if it already exists
    pub async fn fetch_from_url(image_url: ImageUrlObject) -> Result<Self, ImageError> {
        if let Ok(cached_image) = IMAGECACHE.find(&image_url.stem) {
            return Ok(Self {
                stem: image_url.stem,
                format: cached_image.format,
                data: FetchedImageType::Storage(cached_image),
            });
        }

        async fn fetch_bytes(url: Url) -> Result<Bytes, reqwest::Error> {
            let image_result = reqwest::get(url).await?;
            let image_bytes = image_result.bytes().await?;

            Ok(image_bytes)
        }

        match fetch_bytes(image_url.url).await {
            Ok(bytes) => Ok(FetchedImage {
                stem: image_url.stem,
                data: FetchedImageType::Memory(bytes),
                format: image_url.image_format,
            }),
            Err(err) => Err(ImageError::FetchError(err)),
        }
    }
}

#[derive(Debug)]
pub struct ImageUrlObject {
    stem: String,
    url: Url,
    image_format: ImageFormat,
}

impl ImageUrlObject {
    fn get_file_stem(url: &str) -> Result<String, ImageError> {
        match std::path::PathBuf::from_str(url) {
            Ok(path) => path
                .file_name()
                .and_then(|file_name| Some(file_name.to_string_lossy().into_owned()))
                .ok_or(ImageError::InvalidUrl),
            Err(_) => unreachable!(),
        }
    }

    fn get_format(url: &str) -> Result<ImageFormat, ImageError> {
        match ImageFormat::from_path(url) {
            Ok(format) => Ok(format),
            Err(_err) => Err(ImageError::InvalidFormat),
        }
    }

    fn get_url_object(url: &str) -> Result<Url, ImageError> {
        Url::from_str(url).map_err(|_err| ImageError::InvalidUrl)
    }

    pub fn from_str(url: &str) -> Result<Self, ImageError> {
        let stem = Self::get_file_stem(&url)?;
        let image_format = Self::get_format(&url)?;
        let url = Self::get_url_object(&url)?;

        Ok(Self {
            stem,
            image_format,
            url,
        })
    }
}

pub struct ExternalImage<P> {
    path: P,
}

impl<P> ExternalImage<P>
where
    P: AsRef<str>,
{
    pub fn new(path: P) -> Self {
        Self { path }
    }

    pub async fn load(&self) -> Result<SavedImage, ImageError> {
        match self.path.as_ref() {
            url if Url::from_str(url).is_ok_and(|v| ["https", "http"].contains(&v.scheme())) => {
                let image = FetchedImage::fetch_from_url(ImageUrlObject::from_str(url)?).await?;
                IMAGECACHE.cache(&image)
            }
            path if Path::new(path).is_file() => SavedImage::from_path(path),
            _ => Err(ImageError::InvalidExternal),
        }
    }
}
