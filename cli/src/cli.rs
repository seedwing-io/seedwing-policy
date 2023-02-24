use crate::command::bench::Bench;
use crate::command::eval::Eval;
use crate::command::serve::Serve;
use crate::command::verify::Verify;
use clap::ValueEnum;
use std::path::PathBuf;

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
    Test,
}

#[derive(clap::Parser, Debug)]
#[command(
  author,
  version,
  about="Seedwing Policy Tool",
  long_about = None
)]
pub struct Cli {
    #[arg(short, long = "policy", value_name = "DIR")]
    pub(crate) policy_directories: Vec<PathBuf>,

    #[arg(short, long = "data", value_name = "DIR")]
    pub(crate) data_directories: Vec<PathBuf>,

    #[command(subcommand)]
    pub(crate) command: Command,
}

impl Cli {
    pub async fn run(&self) -> Result<(), ()> {
        match &self.command {
            Command::Verify(verify) => {
                verify.run(self).await?;
                println!("ok!");
                Ok(())
            }
            Command::Eval(eval) => eval.run(self).await,
            Command::Bench(bench) => bench.run(self).await,
            Command::Serve(serve) => serve.run(self).await,
            Command::Test => {
                println!("test!");
                Ok(())
            }
        }
    }
}
