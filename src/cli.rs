use clap::{command, Args, Parser, Subcommand};

#[derive(Parser)]
pub struct Options {
    #[arg(short,long,default_value_t=String::from("./cache"),global=true)]
    pub cache_dir: String,

    #[command(subcommand)]
    pub command: SubCommand,

    #[arg(short, long)]
    pub allow_cache_overwrite: bool,
    // #[arg(long = "ssl")]
    // use_ssl: bool,
}

#[derive(Subcommand, Debug)]
pub enum SubCommand {
    // Cache timetable data
    Fetch,
    // Serve data for a frontend via HTTP
    Serve {
        #[arg(long)]
        autofetch: bool,
    },
    // Print timetable oddities
    Verify,
    // Print timetable data
    Print(PrintStruct),
    Bench,
}

#[derive(Debug, Args)]
pub struct PrintStruct {
    #[command(subcommand)]
    pub command: PrintSubCommand,
}

#[derive(Debug, Subcommand)]
pub enum PrintSubCommand {
    Departures { station: String },
}

pub fn get_cli_args() -> Options {
    Options::parse()
}
