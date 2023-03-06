use crate::command::bench::Bench;
use crate::command::eval::Eval;
use crate::command::serve::Serve;
use crate::command::test::Test;
use crate::command::verify::Verify;
use crate::config::Config;
use crate::error::{CliError, ConfigError};
use clap::ValueEnum;
use seedwing_policy_engine::runtime::config::EvalConfig;
use std::path::PathBuf;
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
}

#[derive(clap::Parser, Debug)]
#[command(
author,
version,
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

    #[clap(skip)]
    pub(crate) eval_config: Option<EvalConfig>,
}

impl Cli {
    pub async fn run(&mut self) -> Result<(), CliError> {
        let eval_config = self.load_config_file().await?;
        self.eval_config.replace(eval_config);

        match &self.command {
            Command::Verify(verify) => {
                verify.run(self).await?;
                println!("ok!");
            }
            Command::Eval(eval) => eval.run(self).await?,
            Command::Bench(bench) => bench.run(self).await?,
            Command::Serve(serve) => serve.run(self).await?,
            Command::Test(test) => test.run(self).await?,
        }

        Ok(())
    }

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
            Err(ConfigError::FileNotFound)
        } else {
            Ok(EvalConfig::default())
        }
    }
}
