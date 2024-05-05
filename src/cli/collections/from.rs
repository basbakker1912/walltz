use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail};
use clap::Args;

use crate::collections::Collection;

#[derive(Debug, Clone, Args)]
pub struct FromArgs {
    /// Overwrite the name of the collection if set.
    #[arg(long, short)]
    output_name: Option<String>,
    /// The url of the repository, use SSH for private repositories.
    url: String,
}

impl FromArgs {
    pub fn run(self) -> anyhow::Result<()> {
        let url_path = PathBuf::from(&self.url);

        match url_path.extension() {
            Some(extension) => {
                let ext = extension
                    .to_str()
                    .ok_or(anyhow!("Extension not a string"))?
                    .to_ascii_lowercase();
                match ext.as_str() {
                    "git" => {
                        // TODO: Add a per collection config for naming, tags etc etc
                        match self.output_name {
                            Some(name) => {
                                Collection::clone(&self.url, &name)?;
                            }
                            None => {
                                let path = Path::new(&self.url);
                                let stem = match path.file_stem() {
                                    Some(stem) => stem,
                                    None => bail!("No file stem definied"),
                                }
                                .to_string_lossy();

                                Collection::clone(&self.url, &stem)?;
                            }
                        };

                        Ok(())
                    }
                    _ => bail!("Fetching from this url is not possible"),
                }
            }
            None => todo!("Implement directory importing"),
        }

        // TODO: Add a nice output
    }
}
