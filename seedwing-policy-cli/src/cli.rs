use crate::eval::Eval;
use crate::explain::explain;
use crate::verify::Verify;
use clap::ValueEnum;
use is_terminal::IsTerminal;
use seedwing_policy_engine::runtime::RuntimeError;
use seedwing_policy_engine::value::RuntimeValue;
use std::io::stdin;
use std::path::PathBuf;
use std::process::exit;
use tokio::fs;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
pub enum InputType {
    JSON,
    YAML,
}

#[derive(clap::Subcommand, Debug)]
pub enum Command {
    Verify,
    Eval {
        #[arg(short='t', value_name = "TYPE", value_enum, default_value_t=InputType::JSON)]
        typ: InputType,
        #[arg(short, long)]
        input: Option<PathBuf>,
        #[arg(short = 'n', long = "name")]
        name: String,
    },
    Test,
}

#[derive(clap::Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[arg(short, long = "policy", value_name = "DIR")]
    pub(crate) policy_directories: Vec<PathBuf>,

    #[arg(short, long = "data", value_name = "DIR")]
    pub(crate) data_directories: Vec<PathBuf>,

    #[command(subcommand)]
    pub(crate) command: Command,

    #[arg(short, long = "verbosity", value_name = "LEVEL", default_value_t = 2)]
    pub(crate) verbosity: usize,
}

impl Cli {
    pub async fn run(&self) -> Result<(), ()> {
        match &self.command {
            Command::Verify => {
                let verify = Verify::new(
                    self.policy_directories.clone(),
                    self.data_directories.clone(),
                );

                verify.run().await.map_err(|_| ())?;
                println!("ok!");
            }
            Command::Eval { typ, input, name } => {
                let verify = Verify::new(
                    self.policy_directories.clone(),
                    self.data_directories.clone(),
                );

                let world = verify.run().await.map_err(|_| ())?;

                let value = load_value(*typ, input.clone()).await.map_err(|_| ())?;

                let eval = Eval::new(world, name.clone(), value);

                println!("evaluate pattern: {name}");

                match eval.run().await {
                    Ok(result) => {
                        explain(&result, self.verbosity).unwrap();
                        println!();
                        if result.satisfied() {
                            println!("ok!");
                        } else {
                            println!("pattern match failed");
                            exit(-1);
                        }
                    }
                    Err(e) => {
                        match e {
                            RuntimeError::NoSuchType(name) => {
                                println!("error: no such pattern: {}", name.as_type_str());
                            }
                            _ => {
                                println!("error");
                            }
                        }
                        exit(-10);
                    }
                }
            }
            Command::Test => {
                println!("test!");
            }
        }
        Ok(())
    }
}

pub async fn load_value(
    typ: InputType,
    input: Option<PathBuf>,
) -> Result<RuntimeValue, std::io::Error> {
    if let Some(input) = input {
        let data = fs::read(input).await?;

        match typ {
            InputType::JSON => {
                let value: serde_json::Value = serde_json::from_slice(&*data)?;
                Ok(value.into())
            }
            InputType::YAML => {
                todo!()
            }
        }
    } else {
        if stdin().is_terminal() {
            println!("Enter input value, ^D to finish");
        }
        match typ {
            InputType::JSON => {
                let value: serde_json::Value = serde_json::from_reader(stdin())?;
                Ok(value.into())
            }
            InputType::YAML => {
                todo!()
            }
        }
    }
}
