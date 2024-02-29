use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
pub struct CliOptions {
    #[arg(short,long,default_value_t=String::from("./cache"),global=true)]
    pub cache_dir: String,

    #[command(subcommand)]
    pub command: SubCommand,

    #[arg(short, long)]
    pub allow_cache_overwrite: bool,
    // #[arg(long = "ssl")]
    // use_ssl: bool,
}

#[derive(Subcommand)]
pub enum SubCommand {
    Fetch,
    Serve,
}

pub fn get_cli_args() -> CliOptions {
    CliOptions::parse()
}
