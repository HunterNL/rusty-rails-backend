mod cache;

use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser)]
struct CliOptions {
    #[arg(short,long,default_value_t=String::from("./cache"))]
    cache_dir: String,

    #[command(subcommand)]
    command: SubCommand,

    #[arg(short, long)]
    allow_cache_overwrite: bool,
}

#[derive(Subcommand)]
enum SubCommand {
    Fetch,
    Serve,
}

struct AppConfig {
    ns_api_key: Option<String>,
    cache_dir: PathBuf,
    allow_cache_overwrite: bool,
}
fn main() {
    let cli_options = CliOptions::parse();
    let ns_key = std::env::var("NS_API_KEY");
    let cache_dir: PathBuf = cli_options.cache_dir.into();

    let config: AppConfig = AppConfig {
        cache_dir,
        ns_api_key: ns_key.ok(),
        allow_cache_overwrite: cli_options.allow_cache_overwrite,
    };

    let res: Result<(), String> = match cli_options.command {
        SubCommand::Fetch => cache::update(config),
        SubCommand::Serve => todo!(),
    };

    match res {
        Ok(_) => std::process::exit(0),
        Err(msg) => {
            eprintln!("{}", msg);
            std::process::exit(1)
        }
    }
}
