use std::collections::HashMap;

use anyhow::{anyhow, bail};
use image::ImageFormat;
use rand::Rng;
use serde::Deserialize;
use serde_json::Value;

use super::{ImageUrlObject, SearchParameters};

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

#[derive(Debug, Clone, Deserialize)]
struct ResponseData {
    format: ResponseFormat,
    id: ImageId,
    location: ResponseResultLocation,
    image_url_key: String,
    image_type: ImageTypeDecodeMethod,
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
    pub async fn search(self, parameters: SearchParameters) -> anyhow::Result<ImageUrlObject> {
        let result = reqwest::Client::new()
            .get(self.base_url)
            .query(&[
                self.tags.to_query_entry(parameters.tags),
                self.aspect_ratio.to_query_entry(parameters.aspect_ratios),
                (self.sort.query, self.sort.value),
            ])
            .send()
            .await?;

        let response_data = match self.response.format {
            ResponseFormat::Json => {
                let response: HashMap<String, serde_json::Value> =
                    serde_json::from_slice(result.bytes().await?.as_ref())?;

                let entry = match self.response.location {
                    ResponseResultLocation::Array { key } => {
                        let value = response
                            .get(&key)
                            .ok_or(anyhow!("Key not found in response: {}", key))?;

                        if let serde_json::Value::Array(array) = value {
                            let entry = array
                                .first()
                                .ok_or(anyhow!("No entry in array for key: {}", key))?;

                            entry
                        } else {
                            bail!("Key: {} not of type array", key)
                        }
                    }
                    ResponseResultLocation::Entry { key } => response
                        .get(&key)
                        .ok_or(anyhow!("No value for key: {}", key))?,
                };

                if let serde_json::Value::Object(object) = entry {
                    let image_id = match self.response.id {
                        ImageId::Random => rand::thread_rng().gen::<u32>().to_string(),
                        ImageId::Key { key } => {
                            let image_id = object
                                .get(&key)
                                .ok_or(anyhow!("No value for key: {}", key))?;

                            match image_id {
                                Value::String(value) => value.clone(),
                                Value::Number(value) => value.to_string(),
                                _ => bail!(
                                    "Key for id: {} not of type: String or Number, but of type: {:?}",
                                    key,
                                    image_id
                                )
                            }
                        }
                    };

                    let image_url = {
                        let image_url = object
                            .get(&self.response.image_url_key)
                            .ok_or(anyhow!("No value for key: {}", self.response.image_url_key))?;

                        if let serde_json::Value::String(value) = image_url {
                            value.clone()
                        } else {
                            bail!(
                                "Key for image url: {} not of type: String, but of type: {:?}",
                                self.response.image_url_key,
                                image_url
                            );
                        }
                    };

                    let image_format = match self.response.image_type {
                        ImageTypeDecodeMethod::Path => ImageFormat::from_path(&image_url)?,
                        ImageTypeDecodeMethod::Key { key } => {
                            let image_type = object
                                .get(&key)
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

                    ImageUrlObject {
                        id: image_id,
                        url: image_url,
                        image_format,
                    }
                } else {
                    bail!("Entry should be an object, but is a: {:?}, your data location in the response is incorrect", entry);
                }
            }
        };

        Ok(response_data)
    }
}
