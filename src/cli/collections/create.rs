use regex::Regex;

use crate::collections::Collection;

#[derive(clap::Args, Clone, Debug)]
pub struct CreateCollectionArgs {
    #[arg(short, long)]
    /// Wether to sync the repository through git
    git: bool,
    name: String,
}

impl CreateCollectionArgs {
    pub fn run(self) -> anyhow::Result<()> {
        if self.git {
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

            Collection::create(&self.name, Some(&remote_url))?;
        } else {
            Collection::create(&self.name, None)?;
        };

        println!("Successfully created the collection: {}", self.name);

        Ok(())
    }
}
