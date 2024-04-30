use clap::Args;

use crate::state::State;

#[derive(Args, Debug, Clone)]
pub struct GetArgs {}

impl GetArgs {
    pub async fn run(self) -> anyhow::Result<()> {
        let state = State::load()?;
        let image_path = state.get_current_image()?.get_absolute_path()?;
        println!("{}", image_path.to_string_lossy());
        Ok(())
    }
}
