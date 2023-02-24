use crate::Cli;
use env_logger::Builder;
use log::LevelFilter;
use std::fmt::Arguments;

#[derive(clap::Args, Debug)]
#[command(args_conflicts_with_subcommands = true)]
pub struct Serve {
    #[arg(short, long, default_value = "0.0.0.0")]
    pub(crate) bind: String,

    #[arg(short, long, default_value_t = 8080)]
    pub(crate) port: u16,
}

impl Serve {
    pub async fn run(&self, args: &Cli) -> Result<(), ()> {
        Builder::new()
            .filter_level(LevelFilter::Warn)
            .filter_module("seedwing_policy_server", LevelFilter::Info)
            .filter_module("seedwing_policy_engine", LevelFilter::Info)
            .init();
        seedwing_policy_server::run(
            args.policy_directories.clone(),
            args.data_directories.clone(),
            self.bind.clone(),
            self.port,
        )
        .await
        .map_err(|_| ())
    }
}
