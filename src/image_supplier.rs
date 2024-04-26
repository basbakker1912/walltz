use std::{
    io::Cursor,
    path::{Path, PathBuf},
};

use anyhow::anyhow;
use bytes::Bytes;
use image::ImageFormat;

mod url_supplier;

pub use url_supplier::UrlSupplier;

use crate::BASEDIRECTORIES;

pub struct SearchParameters {
    pub tags: Vec<String>,
    pub aspect_ratios: Vec<String>,
}

pub struct WallpaperImage {
    id: String,
    bytes: Bytes,
    format: ImageFormat,
}

impl WallpaperImage {
    /// Unlike save, this encodes the image correctly. SLOW
    pub fn save_to_format(&self, path: &Path) -> anyhow::Result<()> {
        if self
            .format
            .extensions_str()
            .first()
            .is_some_and(|ext| path.ends_with(ext))
        {
            self.save(path)?;

            return Ok(());
        }

        let reader = image::io::Reader::with_format(Cursor::new(self.bytes.clone()), self.format);
        let image = reader.decode()?;

        image.save(path)?;

        Ok(())
    }

    /// Just saves the file in it's pred
    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        let image_data = self.bytes.as_ref();
        std::fs::write(path, image_data)?;

        Ok(())
    }

    pub fn cache(&self) -> anyhow::Result<PathBuf> {
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
        self.save(&image_path)?;

        Ok(image_path)
    }
}

#[derive(Debug)]
pub struct ImageUrlObject {
    id: String,
    url: String,
    image_format: ImageFormat,
}

pub struct ImageSupplier {
    url_supplier: UrlSupplier,
}

impl ImageSupplier {
    async fn get_image(&self, image_url: ImageUrlObject) -> anyhow::Result<WallpaperImage> {
        let image_result = reqwest::get(&image_url.url).await?;
        let image_bytes = image_result.bytes().await?;

        Ok(WallpaperImage {
            id: image_url.id,
            bytes: image_bytes,
            format: image_url.image_format,
        })
    }

    pub async fn get_wallpaper_image(
        &self,
        parameters: SearchParameters,
    ) -> anyhow::Result<WallpaperImage> {
        let image_url = self.url_supplier.clone().search(parameters).await?;
        let image = self.get_image(image_url).await?;

        Ok(image)
    }

    pub fn new(url_supplier: UrlSupplier) -> Self {
        Self { url_supplier }
    }
}
