use std::path::PathBuf;
use clap::ValueEnum;

pub const COMMAND_NAME: &str = "seedwing-policy";

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
pub enum InputType {
    JSON,
    YAML,
}

#[derive(clap::Subcommand, Debug)]
pub enum Command {
    Validate,
    Eval{
        #[arg(short='t', value_name = "TYPE", value_enum, default_value_t=InputType::JSON)]
        typ: InputType,
        #[arg(short, long)]
        input: Option<PathBuf>
    },
    Test,
}

#[derive(clap::Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[arg(short, long)]
    pub(crate) policy_directory: Vec<PathBuf>,

    #[arg(short, long)]
    pub(crate) data_directory: Vec<PathBuf>,

    #[command(subcommand)]
    pub(crate) command: Command,
}

impl Cli {

    pub async fn run(&self) -> Result<(), ()> {
        match self.command {
            Command::Validate => {
                println!("validate!");
            }
            Command::Eval { .. } => {
                println!("eval!");
            }
            Command::Test => {
                println!("test!");
            }
        }
        Ok(())
    }

}