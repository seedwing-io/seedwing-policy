use crate::cli::Context;
use seedwing_policy_engine::runtime::World;

#[derive(clap::Args, Debug)]
#[command(
    about = "Verify compilation of patterns",
    args_conflicts_with_subcommands = true
)]
pub struct Verify {}

impl Verify {
    pub async fn run(&self, context: Context) -> anyhow::Result<World> {
        Ok(context.world().await?.1)
    }
}
