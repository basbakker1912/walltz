use std::{collections::HashMap, str::FromStr};

use anyhow::{anyhow, bail};
use image::ImageFormat;
use rand::Rng;
use reqwest::{Response, Url};
use serde::Deserialize;
use serde_json::Value;

use crate::IMAGECACHE;

use super::{ImageUrl, SearchParameters};

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
enum ImageId {
    #[serde(alias = "key", alias = "KEY")]
    Key { key: String },
    #[serde(alias = "random", alias = "RANDOM")]
    Random,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
enum ImageTypeDecodeMethod {
    #[serde(alias = "key", alias = "KEY")]
    Key { key: String },
    #[serde(alias = "path", alias = "PATH")]
    Path,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
enum ResponseResultLocation {
    #[serde(alias = "array", alias = "ARRAY")]
    Array { key: String },
    #[serde(alias = "entry", alias = "ENTRY")]
    Entry { key: String },
}

#[derive(Debug, Clone, Deserialize)]
enum ResponseFormat {
    #[serde(alias = "json", alias = "JSON")]
    Json,
}

struct JsonResponseDecoder {
    id: ImageId,
    location: ResponseResultLocation,
    image_url_key: String,
    image_type: ImageTypeDecodeMethod,
}

impl JsonResponseDecoder {
    fn decode_entry(&self, entry: serde_json::Value) -> anyhow::Result<ImageUrl> {
        if let serde_json::Value::Object(object) = entry {
            let image_id = match &self.id {
                ImageId::Random => rand::thread_rng().gen::<u32>().to_string(),
                ImageId::Key { key } => {
                    let image_id = object
                        .get(key)
                        .ok_or(anyhow!("No value for key: {}", key))?;

                    match image_id {
                        Value::String(value) => value.clone(),
                        Value::Number(value) => value.to_string(),
                        _ => bail!(
                            "Key for id: {} not of type: String or Number, but of type: {:?}",
                            key,
                            image_id
                        ),
                    }
                }
            };

            let image_url = {
                let image_url = object
                    .get(&self.image_url_key)
                    .ok_or(anyhow!("No value for key: {}", self.image_url_key))?;

                if let serde_json::Value::String(value) = image_url {
                    value.clone()
                } else {
                    bail!(
                        "Key for image url: {} not of type: String, but of type: {:?}",
                        self.image_url_key,
                        image_url
                    );
                }
            };

            let image_format = match &self.image_type {
                ImageTypeDecodeMethod::Path => ImageFormat::from_path(&image_url)?,
                ImageTypeDecodeMethod::Key { key } => {
                    let image_type = object
                        .get(key)
                        .ok_or(anyhow!("No value for key: {}", key))?;

                    if let serde_json::Value::String(value) = image_type {
                        ImageFormat::from_mime_type(value).ok_or(anyhow!(
                            "No valid file format for mime type: {}",
                            image_type
                        ))?
                    } else {
                        bail!(
                            "Key for image type: {} not of type: String, but of type: {:?}",
                            key,
                            image_type
                        );
                    }
                }
            };

            Ok(ImageUrl {
                stem: image_id,
                url: Url::from_str(&image_url)?,
                image_format,
            })
        } else {
            todo!("Implement a thing here")
        }
    }

    fn decode_base(
        &self,
        data: HashMap<String, serde_json::Value>,
        parameters: &SearchParameters,
    ) -> anyhow::Result<ImageUrl> {
        match &self.location {
            ResponseResultLocation::Array { key } => {
                let value = data
                    .get(key)
                    .ok_or(anyhow!("Key not found in response: {}", key))?;

                if let serde_json::Value::Array(entries) = value {
                    if parameters.skip_cache {
                        for entry in entries {
                            let decoded_entry = self.decode_entry(entry.to_owned())?;
                            let cached_image = IMAGECACHE.find(&decoded_entry.stem);

                            if cached_image.is_ok() {
                                continue;
                            }

                            return Ok(decoded_entry);
                        }
                    }

                    let entry = entries
                        .first()
                        .ok_or(anyhow::anyhow!("No entries in response array"))?;

                    let decoded_entry = self.decode_entry(entry.to_owned())?;

                    Ok(decoded_entry)
                } else {
                    bail!("Key: {} not of type array", key)
                }
            }
            ResponseResultLocation::Entry { key } => {
                let entry = data.get(key).ok_or(anyhow!("No value for key: {}", key))?;
                let decoded_entry = self.decode_entry(entry.to_owned())?;

                Ok(decoded_entry)
            }
        }
    }

    pub fn decode(
        &self,
        response: reqwest::blocking::Response,
        parameters: &SearchParameters,
    ) -> anyhow::Result<ImageUrl> {
        let data: HashMap<String, serde_json::Value> =
            serde_json::from_slice(response.bytes()?.as_ref())?;

        self.decode_base(data, parameters)
    }
}

impl From<ResponseData> for JsonResponseDecoder {
    fn from(value: ResponseData) -> Self {
        Self {
            id: value.id,
            location: value.location,
            image_url_key: value.image_url_key,
            image_type: value.image_type,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct ResponseData {
    format: ResponseFormat,
    id: ImageId,
    location: ResponseResultLocation,
    image_url_key: String,
    image_type: ImageTypeDecodeMethod,
}

impl ResponseData {
    fn process_response(
        self,
        response: reqwest::blocking::Response,
        parameters: &SearchParameters,
    ) -> anyhow::Result<ImageUrl> {
        match self.format {
            ResponseFormat::Json => JsonResponseDecoder::from(self).decode(response, parameters),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct QueryData {
    query: String,
    prefix: Option<String>,
    seperator: Option<String>,
}

impl QueryData {
    pub fn to_query_entry(self, data: Vec<String>) -> (String, String) {
        let query_data = data
            .into_iter()
            .map(|entry| {
                if let Some(mut prefix) = self.prefix.clone() {
                    prefix.push_str(&entry);
                    prefix
                } else {
                    entry
                }
            })
            .collect::<Vec<_>>()
            .join(&self.seperator.unwrap_or_default());

        (self.query, query_data)
    }
}

#[derive(Debug, Clone, Deserialize)]
struct SortData {
    query: String,
    value: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UrlSupplier {
    base_url: String,
    response: ResponseData,
    tags: QueryData,
    aspect_ratio: QueryData,
    sort: SortData,
}

impl UrlSupplier {
    pub fn search(self, parameters: SearchParameters) -> anyhow::Result<ImageUrl> {
        let result = {
            let parameters = parameters.clone();
            reqwest::blocking::Client::new()
                .get(self.base_url)
                .query(&[
                    self.tags.to_query_entry(parameters.tags),
                    self.aspect_ratio.to_query_entry(parameters.aspect_ratios),
                    (self.sort.query, self.sort.value),
                ])
                .send()?
        };

        let response_data = self.response.process_response(result, &parameters)?;

        Ok(response_data)
    }
}

// TODO: This implementation is so fucking ugly and slow, should redo
// Also use state to keep track of position in array, and step through it, add pagination support for API's aswell
