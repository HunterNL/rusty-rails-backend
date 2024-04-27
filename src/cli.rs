use clap::{Parser, Subcommand};

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

#[derive(Subcommand)]
pub enum SubCommand {
    Fetch,
    Serve,
}

pub fn get_cli_args() -> Options {
    Options::parse()
}
