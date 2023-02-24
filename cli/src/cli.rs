use crate::eval::Eval;
use crate::explain::explain;
use crate::verify::Verify;
use clap::ValueEnum;
use is_terminal::IsTerminal;
use seedwing_policy_engine::runtime::RuntimeError;
use seedwing_policy_engine::value::RuntimeValue;
use serde_yaml::Error as YamlError;
use std::io::stdin;
use std::path::PathBuf;
use std::process::exit;
use tokio::fs;
use tokio::time::Instant;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
pub enum InputType {
    Json,
    Yaml,
}

#[derive(clap::Subcommand, Debug)]
pub enum Command {
    Verify,
    Eval {
        #[arg(short='t', value_name = "TYPE", value_enum, default_value_t=InputType::Json)]
        typ: InputType,
        #[arg(short, long)]
        input: Option<PathBuf>,
        #[arg(short = 'n', long = "name")]
        name: String,
    },
    Bench {
        #[arg(short='t', value_name = "TYPE", value_enum, default_value_t=InputType::Json)]
        typ: InputType,
        #[arg(short, long)]
        input: Option<PathBuf>,
        #[arg(short = 'n', long = "name")]
        name: String,
        #[arg(short = 'i', long = "iterations")]
        iterations: usize,
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
                        explain(&result).unwrap();
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
            Command::Bench {
                typ,
                input,
                name,
                iterations,
            } => {
                let verify = Verify::new(
                    self.policy_directories.clone(),
                    self.data_directories.clone(),
                );

                let world = verify.run().await.map_err(|_| ())?;

                let value = load_value(*typ, input.clone()).await.map_err(|_| ())?;

                let eval = Eval::new(world, name.clone(), value);

                use hdrhistogram::Histogram;
                let mut hist = Histogram::<u64>::new(2).unwrap();

                // Warm up for 1/10th of the iterations
                for _iter in 0..(*iterations / 10) {
                    match eval.run().await {
                        Ok(_result) => {}
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

                for _iter in 0..*iterations {
                    let start = Instant::now();
                    match eval.run().await {
                        Ok(_result) => {
                            let end = Instant::now();
                            let duration = end - start;
                            hist.record(duration.as_nanos() as u64).unwrap();
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

                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json! ({
                        "samples": hist.len(),
                        "latency": {
                            "avg": hist.mean() as u64,
                            "stdev": hist.stdev() as u64,
                            "min": hist.min() as u64,
                            "max": hist.max() as u64,
                            "p50": hist.value_at_quantile(0.50) as u64,
                            "p99": hist.value_at_quantile(0.99) as u64,
                        }
                    }))
                    .unwrap()
                );
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
            InputType::Json => {
                let value: serde_json::Value = serde_json::from_slice(&data)?;
                Ok(value.into())
            }
            InputType::Yaml => {
                let value: serde_json::Value = serde_yaml::from_slice(&data)
                    .map_err(YamlError::from)
                    .unwrap();
                Ok(value.into())
            }
        }
    } else {
        if stdin().is_terminal() {
            println!("Enter input value, ^D to finish");
        }
        match typ {
            InputType::Json => {
                let value: serde_json::Value = serde_json::from_reader(stdin())?;
                Ok(value.into())
            }
            InputType::Yaml => {
                let value: serde_json::Value = serde_yaml::from_reader(stdin())
                    .map_err(YamlError::from)
                    .unwrap();
                Ok(value.into())
            }
        }
    }
}
