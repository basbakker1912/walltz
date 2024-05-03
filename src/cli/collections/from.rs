use std::path::PathBuf;

use anyhow::{anyhow, bail};
use clap::Args;

#[derive(Debug, Clone, Args)]
pub struct FromArgs {
    /// Overwrite the name of the collection if set.
    #[arg(long, short)]
    output_name: Option<String>,
    /// The url of the repository, use SSH for private repositories.
    url: String,
}

impl FromArgs {
    pub async fn run(self) -> anyhow::Result<()> {
        let url_path = PathBuf::from(&self.url);

        match url_path.extension() {
            Some(extension) => {
                let ext = extension
                    .to_str()
                    .ok_or(anyhow!("Extension not a string"))?
                    .to_ascii_lowercase();
                match ext.as_str() {
                    "git" => {
                        todo!("Add git cloning support")
                        // Collection::clone_from_git(&self.url, self.output_name)?;
                    }
                    _ => bail!("Fetching from this url is not possible"),
                }
            }
            None => todo!("Implement directory importing"),
        }
    }
}
