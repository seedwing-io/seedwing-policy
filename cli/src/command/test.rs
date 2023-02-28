use crate::command::verify::Verify;
use crate::Cli;
use seedwing_policy_engine::runtime::{EvalContext, Output, PatternName, RuntimeError, World};
use seedwing_policy_engine::value::RuntimeValue;
use serde_json::Value;
use std::fmt::{Display, Formatter};
use std::path::PathBuf;
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use walkdir::{DirEntry, WalkDir};

#[derive(clap::Args, Debug)]
#[command(about = "Execute benchmarks", args_conflicts_with_subcommands = true)]
pub struct Test {
    #[arg(short, long = "test", value_name = "DIR")]
    pub(crate) test_directories: Vec<PathBuf>,

    #[arg(short, long, value_name = "PATTERN_MATCH")]
    pub(crate) pattern: Option<String>,
}

impl Test {
    pub async fn run(&self, args: &Cli) -> Result<(), ()> {
        let world = Verify::verify(args).await?;
        let plan = TestPlan::new(&self.test_directories);
        self.display_results(plan.run(&world).await);

        Ok(())
    }

    pub fn display_results(&self, results: Vec<(TestRunner, TestResult)>) {
        let mut last_pattern = None;
        let mut new_pattern = false;
        for (test, result) in results {
            if let Some(prev) = &last_pattern {
                if *prev != test.pattern.as_type_str() {
                    new_pattern = true
                }
            } else if last_pattern.is_none() {
                new_pattern = true
            }
            if new_pattern {
                println!("{}", test.pattern.as_type_str());
            }
            last_pattern.replace(test.pattern.as_type_str());
            new_pattern = false;
            println!("  {} - {}", test.name, result);
        }
    }
}

#[derive(Debug)]
pub struct TestPlan {
    tests: Vec<TestRunner>,
}

impl TestPlan {
    pub fn new(dirs: &Vec<PathBuf>) -> Self {
        let mut tests = dirs
            .iter()
            .flat_map(|dir| {
                WalkDir::new(dir)
                    .into_iter()
                    .filter_map(|entry| entry.ok())
                    .flat_map(move |e: DirEntry| {
                        let name = e.file_name().to_string_lossy();
                        if name == "input.json" {
                            if let Ok(path) = e.path().strip_prefix(dir) {
                                let test_name = path.parent().map(|e| e.file_name());
                                let pattern_name = path.parent().map(|e| e.parent());

                                match (pattern_name, test_name) {
                                    (Some(Some(pattern_name)), Some(Some(test_name))) => {
                                        let expected = if let Some(parent) = e.path().parent() {
                                            if parent.join("output.json").exists() {
                                                Expected::Transform(parent.join("output.json"))
                                            } else if parent.join("IDENTITY").exists() {
                                                Expected::Identity
                                            } else {
                                                Expected::None
                                            }
                                        } else {
                                            Expected::None
                                        };

                                        Some(TestRunner {
                                            name: (*test_name.to_string_lossy()).into(),
                                            pattern: (pattern_name
                                                .to_string_lossy()
                                                .replace("/", "::"))
                                            .into(),
                                            input: e.path().into(),
                                            expected,
                                        })
                                    }
                                    _ => None,
                                }
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    })
            })
            .collect::<Vec<TestRunner>>();

        tests.sort_by(|l, r| l.pattern.as_type_str().cmp(&r.pattern.as_type_str()));

        Self { tests }
    }

    pub async fn run(&self, world: &World) -> Vec<(TestRunner, TestResult)> {
        let mut results = Vec::new();
        for test in &self.tests {
            results.push((test.clone(), test.run(world).await));
        }
        results
    }
}

#[derive(Debug, Clone)]
pub struct TestRunner {
    name: String,
    pattern: PatternName,
    input: PathBuf,
    expected: Expected,
}

impl TestRunner {
    pub async fn run(&self, world: &World) -> TestResult {
        if let Ok(mut input_file) = File::open(&self.input).await {
            let mut input = Vec::new();
            let read_result = input_file.read_to_end(&mut input).await;
            if read_result.is_ok() {
                let input: Result<Value, _> = serde_json::from_slice(&*input);
                if let Ok(input) = input {
                    let result = world
                        .evaluate(self.pattern.as_type_str(), input, EvalContext::default())
                        .await;

                    match result {
                        Ok(result) => match (result.raw_output(), &self.expected) {
                            (Output::None, Expected::None) => TestResult::Passed,
                            (Output::Identity, Expected::Identity) => TestResult::Passed,
                            (Output::Transform(val), Expected::Transform(expected_val)) => {
                                if let Ok(mut output_file) = File::open(expected_val).await {
                                    let mut output = Vec::new();
                                    let read_result = output_file.read_to_end(&mut output).await;
                                    if read_result.is_ok() {
                                        let output: Result<Value, _> =
                                            serde_json::from_slice(&*output);
                                        if let Ok(output) = output {
                                            let output: RuntimeValue = output.into();

                                            if *val.as_ref() == output {
                                                return TestResult::Passed;
                                            }
                                        }
                                    }
                                }
                                TestResult::Failed
                            }
                            _ => TestResult::Failed,
                        },
                        Err(err) => TestResult::Error(TestError::Runtime(err)),
                    }
                } else {
                    TestResult::Error(TestError::Deserialization)
                }
            } else {
                TestResult::Error(TestError::ReadingInput)
            }
        } else {
            TestResult::Error(TestError::ReadingInput)
        }
    }
}

#[derive(Debug, Clone)]
pub enum Expected {
    Identity,
    Transform(PathBuf),
    None,
}

#[derive(Debug)]
pub enum TestResult {
    Passed,
    Failed,
    Error(TestError),
}

impl Display for TestResult {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            TestResult::Passed => write!(f, "passed"),
            TestResult::Failed => write!(f, "failed"),
            TestResult::Error(err) => write!(f, "error: {:?}", err),
        }
    }
}

#[derive(Debug)]
pub enum TestError {
    ReadingInput,
    Deserialization,
    Runtime(RuntimeError),
}
