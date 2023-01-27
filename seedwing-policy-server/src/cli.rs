use clap::{value_parser, Arg, Command};

pub const COMMAND_NAME: &str = "seedwing-policy";

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
