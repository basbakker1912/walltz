use clap::Args;
use regex::Regex;

use crate::collections::Collection;

#[derive(Debug, Clone, Args)]
pub struct SyncArgs {
    /// The name of the collection to sync.
    name: String,
}

impl SyncArgs {
    pub fn run(self) -> anyhow::Result<()> {
        let mut collection = Collection::open(&self.name)?;

        let repository = if let Some(repository) = collection.get_repository() {
            let commit_message: String = dialoguer::Input::new()
                .with_prompt("Commit message")
                .interact_text()?;

            repository.commit_all(&commit_message)?;

            repository
        } else {
            let check_regex = Regex::new(
                r#"((git|ssh|http(s)?)|(git@[\w\.]+))(:(//)?)([\w\.@\:/\-~]+)(\.git)(/)?"#,
            )?;
            let remote_url: String = dialoguer::Input::new()
                .with_prompt("Please specify the remote repository url")
                .validate_with(|value: &String| -> Result<(), &str> {
                    if check_regex.is_match(&value) {
                        Ok(())
                    } else {
                        Err("Not a valid git repository")
                    }
                })
                .interact_text()?;

            collection
                .get_directory_mut()
                .initialize_repository(&remote_url)?
        };

        repository.sync()?;

        Ok(())
    }
}
