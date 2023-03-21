use crate::cli::{Context, InputType};
use crate::util;
use crate::util::load_value;
use seedwing_policy_engine::runtime::Response;
use std::path::PathBuf;
use std::process::ExitCode;

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
    #[arg(short = 'v', long = "verbose", default_value_t = false)]
    verbose: bool,
}

impl Eval {
    pub async fn run(&self, context: Context) -> anyhow::Result<ExitCode> {
        let world = context.world().await?.1;

        let value = load_value(self.typ, self.input.clone()).await?;
        let eval = util::eval::Eval::new(world, self.name.clone(), value);

        println!("evaluate pattern: {}", self.name);

        let result = eval.run().await?;
        let response = if self.verbose {
            Response::new(&result)
        } else {
            Response::new(&result).collapse()
        };

        println!("{}", serde_json::to_string_pretty(&response).unwrap());
        if !result.satisfied() {
            return Ok(ExitCode::from(2));
        }

        Ok(ExitCode::SUCCESS)
    }
}
