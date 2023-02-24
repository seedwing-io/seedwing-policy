use crate::cli::InputType;
use crate::command::verify::Verify;
use crate::util;
use crate::util::explain::explain;
use crate::util::load_value;
use crate::Cli;
use seedwing_policy_engine::runtime::RuntimeError;
use std::path::PathBuf;
use std::process::exit;

#[derive(clap::Args, Debug)]
#[command(
    about = "Evaluate a pattern against an input",
    args_conflicts_with_subcommands = true
)]
pub struct Eval {
    #[arg(short='t', value_name = "TYPE", value_enum, default_value_t=InputType::Json)]
    typ: InputType,
    #[arg(short, long)]
    input: Option<PathBuf>,
    #[arg(short = 'n', long = "name")]
    name: String,
}

impl Eval {
    pub async fn run(&self, args: &Cli) -> Result<(), ()> {
        let world = Verify::verify(args).await?;

        let value = load_value(self.typ, self.input.clone())
            .await
            .map_err(|_| ())?;

        let eval = util::eval::Eval::new(world, self.name.clone(), value);

        println!("evaluate pattern: {}", self.name);

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
        Ok(())
    }
}
