use crate::Cli;
use seedwing_policy_engine::data::DirectoryDataSource;
use seedwing_policy_engine::lang::builder::Builder;
use seedwing_policy_engine::runtime::sources::Directory;
use seedwing_policy_engine::runtime::ErrorPrinter;
use seedwing_policy_engine::runtime::World;
use std::path::PathBuf;
use std::process::exit;

#[derive(clap::Args, Debug)]
#[command(
    about = "Verify compilation of patterns",
    args_conflicts_with_subcommands = true
)]
pub struct Verify {}

impl Verify {
    pub async fn verify(args: &Cli) -> Result<World, ()> {
        Verify {}.run(args).await
    }

    pub async fn verify_with_builder(args: &Cli) -> Result<(Builder, World), ()> {
        let verify = Verify {};
        let mut builder = verify.builder(args).await?;
        let result = builder.finish().await;

        match result {
            Ok(world) => Ok((builder, world)),
            Err(errors) => {
                ErrorPrinter::new(builder.source_cache()).display(&errors);
                exit(-2);
            }
        }
    }

    pub async fn run(&self, args: &Cli) -> Result<World, ()> {
        let mut builder = self.builder(args).await?;
        let result = builder.finish().await;

        match result {
            Ok(world) => Ok(world),
            Err(errors) => {
                ErrorPrinter::new(builder.source_cache()).display(&errors);
                exit(-2);
            }
        }
    }

    pub async fn builder(&self, args: &Cli) -> Result<Builder, ()> {
        let mut errors = Vec::new();

        let mut builder = if let Some(config) = args.eval_config.as_ref() {
            Builder::new_with_config(config.clone())
        } else {
            Builder::new()
        };

        let mut sources = Vec::new();
        for dir in &args.policy_directories {
            let dir = PathBuf::from(dir);
            if !dir.exists() {
                log::error!("Unable to open directory: {}", dir.to_string_lossy());
                exit(-3);
            }
            sources.push(Directory::new(dir));
        }

        //log::info!("loading policies from {}", dir);
        for source in sources.iter() {
            if let Err(result) = builder.build(source.iter()) {
                errors.extend_from_slice(&result);
            }
        }

        if !errors.is_empty() {
            ErrorPrinter::new(builder.source_cache()).display(&errors);
            exit(-1)
        }

        for each in &args.data_directories {
            log::info!("loading data from {:?}", each);
            builder.data(DirectoryDataSource::new(each.into()));
        }

        Ok(builder)
    }
}
