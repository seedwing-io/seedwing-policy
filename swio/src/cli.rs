use crate::command::bench::Bench;
use crate::command::docs::Docs;
use crate::command::eval::Eval;
use crate::command::serve::Serve;
use crate::command::test::Test;
use crate::command::verify::Verify;
use crate::config::Config;
use crate::error::ConfigError;
use anyhow::bail;
use clap::ValueEnum;
use seedwing_policy_engine::data::DirectoryDataSource;
use seedwing_policy_engine::lang::builder::Builder;
use seedwing_policy_engine::runtime::config::EvalConfig;
use seedwing_policy_engine::runtime::sources::Directory;
use seedwing_policy_engine::runtime::{ErrorPrinter, World};
use std::path::PathBuf;
use std::process::{ExitCode, Termination};
use std::str::from_utf8;
use tokio::fs::File;
use tokio::io::AsyncReadExt;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
pub enum InputType {
    Json,
    Yaml,
}

#[derive(clap::Subcommand, Debug)]
pub enum Command {
    Verify(Verify),
    Eval(Eval),
    Bench(Bench),
    Serve(Serve),
    Test(Test),
    Docs(Docs),
}

#[derive(clap::Parser, Debug)]
#[command(
    author,
    version = seedwing_policy_engine::version(),
    about = "Seedwing Policy Tool",
    long_about = None
)]
pub struct Cli {
    #[arg(short = 'f', long = "config", value_name = "FILE", global = true)]
    pub(crate) config_file: Option<PathBuf>,

    #[arg(short, long = "policy", value_name = "DIR", global = true)]
    pub(crate) policy_directories: Vec<PathBuf>,

    #[arg(short, long = "data", value_name = "DIR", global = true)]
    pub(crate) data_directories: Vec<PathBuf>,

    #[command(subcommand)]
    pub(crate) command: Command,
}

pub struct Context {
    pub config_file: Option<PathBuf>,

    pub policy_directories: Vec<PathBuf>,

    pub data_directories: Vec<PathBuf>,

    pub eval_config: Option<EvalConfig>,
}

impl Cli {
    pub async fn run(self) -> ExitCode {
        match self.run_command().await {
            Ok(code) => code,
            Err(err) => {
                eprintln!("{err}");
                ExitCode::FAILURE
            }
        }
    }

    async fn run_command(self) -> anyhow::Result<ExitCode> {
        let mut context = Context {
            config_file: self.config_file,
            policy_directories: self.policy_directories,
            data_directories: self.data_directories,
            eval_config: None,
        };

        let eval_config = context.load_config_file().await?;
        context.eval_config.replace(eval_config);

        Ok(match self.command {
            Command::Verify(verify) => {
                verify.run(context).await?;
                println!("ok!");
                ExitCode::SUCCESS
            }
            Command::Eval(eval) => eval.run(context).await?,
            Command::Bench(bench) => bench.run(context).await?,
            Command::Serve(serve) => serve.run(context).await?.report(),
            Command::Test(test) => test.run(context).await?,
            Command::Docs(docs) => docs.run(context).await?.report(),
        })
    }
}

impl Context {
    async fn load_config_file(&mut self) -> Result<EvalConfig, ConfigError> {
        let (explicit, path) = if let Some(path) = &self.config_file {
            if path.is_dir() {
                (true, path.join("Seedwing.toml"))
            } else {
                (true, path.clone())
            }
        } else {
            (false, String::from("Seedwing.toml").into())
        };

        if path.exists() {
            if let Ok(mut config_file) = File::open(&path).await {
                let mut config = Vec::new();
                let read_result = config_file.read_to_end(&mut config).await;
                if read_result.is_ok() {
                    if let Ok(toml) = from_utf8(&config) {
                        let config: Config = toml::from_str(toml)?;
                        println!("{:?}", config);
                        if let Some(parent) = path.parent() {
                            let policy_dirs = config.policy_directories(parent);
                            self.policy_directories.extend_from_slice(&policy_dirs);
                            let data_dirs = config.data_directories(parent);
                            self.data_directories.extend_from_slice(&data_dirs);
                        }
                        Ok(config.eval_config())
                    } else {
                        Err(ConfigError::InvalidFormat)
                    }
                } else {
                    Err(ConfigError::NotReadable)
                }
            } else {
                Err(ConfigError::NotReadable)
            }
        } else if explicit {
            Err(ConfigError::FileNotFound(path))
        } else {
            Ok(EvalConfig::default())
        }
    }

    pub async fn builder(&self) -> anyhow::Result<Builder> {
        let mut errors = Vec::new();

        let mut builder = if let Some(config) = &self.eval_config {
            Builder::new_with_config(config.clone())
        } else {
            Builder::new()
        };

        let mut sources = Vec::new();
        for dir in &self.policy_directories {
            let dir = PathBuf::from(dir);
            if !dir.exists() {
                log::error!("Unable to open directory: {}", dir.to_string_lossy());
                return Err(ConfigError::FileNotFound(dir).into());
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
            bail!("Failed to load patterns");
        }

        for each in &self.data_directories {
            log::info!("loading data from {:?}", each);
            builder.data(DirectoryDataSource::new(each.into()));
        }

        Ok(builder)
    }

    pub async fn world(&self) -> anyhow::Result<(Builder, World)> {
        let mut builder = self.builder().await?;
        let result = builder.finish().await;

        match result {
            Ok(world) => Ok((builder, world)),
            Err(errors) => {
                ErrorPrinter::new(builder.source_cache()).display(&errors);
                bail!("Failed to build world");
            }
        }
    }
}
