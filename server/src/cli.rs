use clap::ValueEnum;
use log::LevelFilter;
use std::path::PathBuf;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
pub enum LogLevel {
    Off,
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl From<LogLevel> for LevelFilter {
    fn from(level: LogLevel) -> Self {
        match level {
            LogLevel::Trace => LevelFilter::Trace,
            LogLevel::Debug => LevelFilter::Debug,
            LogLevel::Info => LevelFilter::Info,
            LogLevel::Warn => LevelFilter::Warn,
            LogLevel::Error => LevelFilter::Error,
            LogLevel::Off => LevelFilter::Off,
        }
    }
}

#[derive(clap::Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[arg(short, long, default_value = "0.0.0.0")]
    pub(crate) bind: String,

    #[arg(short = 'P', long, default_value_t = 8080)]
    pub(crate) port: u16,

    #[arg(short, long = "policy", value_name = "DIR")]
    pub(crate) policy_directories: Vec<PathBuf>,

    #[arg(short, long = "data", value_name = "DIR")]
    pub(crate) data_directories: Vec<PathBuf>,

    #[arg(long, value_name = "LEVEL", value_enum, default_value_t=LogLevel::Info)]
    pub(crate) log: LogLevel,
}

/*
pub fn cli() -> Command {
    Command::new(COMMAND_NAME)
        .arg(
            Arg::new("bind")
                .long("bind")
                .short('b')
                .default_value("0.0.0.0")
                .value_name("bind address"),
        )
        .arg(
            Arg::new("port")
                .long("port")
                .short('p')
                .value_name("listen port")
                .value_parser(value_parser!(u16))
                .default_value("8080"),
        )
        .arg(
            Arg::new("log")
                .long("log")
                .value_name("level")
                .default_value("info"),
        )
        .arg(
            Arg::new("data")
                .long("data")
                .short('d')
                .value_name("data directory")
                .num_args(0..),
        )
        .arg(Arg::new("dir").value_name("policy directory").num_args(1..))
}
 */
