use crate::cli::{Context, InputType};
use crate::util::{self, load_values};
use seedwing_policy_engine::runtime::RuntimeError;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Instant;

#[derive(clap::Args, Debug)]
#[command(about = "Execute benchmarks", args_conflicts_with_subcommands = true)]
pub struct Bench {
    #[arg(short = 't', value_name = "TYPE", value_enum, default_value_t = InputType::Json)]
    typ: InputType,
    #[arg(short, long)]
    input: PathBuf,
    #[arg(short = 'n', long = "name")]
    name: String,
    #[arg(short = 'c', long = "count")]
    count: usize,
}

impl Bench {
    pub async fn run(&self, context: Context) -> anyhow::Result<ExitCode> {
        let world = context.world().await?.1;

        let value = load_values(self.typ, vec![self.input.clone()]).await?[0].clone();
        let eval = util::eval::Eval::new(&world, &self.name, value);

        use hdrhistogram::Histogram;
        let mut hist = Histogram::<u64>::new(2).unwrap();

        // Warm up for 1/10th of the iterations
        for _iter in 0..(self.count / 10) {
            match eval.run().await {
                Ok(_result) => {}
                Err(e) => {
                    match e {
                        RuntimeError::NoSuchPattern(name) => {
                            println!("error: no such pattern: {}", name.as_type_str());
                        }
                        _ => {
                            println!("error");
                        }
                    }
                    return Ok(ExitCode::from(10));
                }
            }
        }

        for _iter in 0..self.count {
            let start = Instant::now();
            match eval.run().await {
                Ok(_result) => {
                    let end = Instant::now();
                    let duration = end - start;
                    hist.record(duration.as_nanos() as u64).unwrap();
                }
                Err(e) => {
                    match e {
                        RuntimeError::NoSuchPattern(name) => {
                            println!("error: no such pattern: {}", name.as_type_str());
                        }
                        _ => {
                            println!("error");
                        }
                    }
                    return Ok(ExitCode::from(10));
                }
            }
        }

        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "samples": hist.len(),
                "latency": {
                    "avg": hist.mean(),
                    "stdev": hist.stdev(),
                    "min": hist.min(),
                    "max": hist.max(),
                    "p50": hist.value_at_quantile(0.50),
                    "p99": hist.value_at_quantile(0.99),
                }
            }))
            .unwrap()
        );

        return Ok(ExitCode::SUCCESS);
    }
}
