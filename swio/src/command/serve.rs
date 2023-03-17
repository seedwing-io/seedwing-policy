use crate::Cli;
use env_logger::Builder;
use log::LevelFilter;
use seedwing_policy_engine::data::{DataSource, DirectoryDataSource};

#[derive(clap::Args, Debug)]
#[command(
    about = "Launch an API and UI server",
    args_conflicts_with_subcommands = true
)]
pub struct Serve {
    #[arg(short, long, default_value = "0.0.0.0")]
    pub(crate) bind: String,

    #[arg(short = 'P', long = "port", default_value_t = 8080)]
    pub(crate) port: u16,
}

impl Serve {
    pub async fn run(&self, args: &Cli) -> Result<(), ()> {
        Builder::new()
            .filter_level(LevelFilter::Warn)
            .filter_module("seedwing_policy_server", LevelFilter::Info)
            .filter_module("seedwing_policy_engine", LevelFilter::Info)
            .init();

        let mut data_directories: Vec<Box<dyn DataSource>> = Vec::new();
        for each in args.data_directories.iter() {
            log::info!("loading data from {:?}", each);
            data_directories.push(Box::new(DirectoryDataSource::new(each.into())));
        }

        seedwing_policy_server::run(
            args.policy_directories.clone(),
            data_directories,
            self.bind.clone(),
            self.port,
        )
        .await
        .map_err(|_| ())
    }
}
