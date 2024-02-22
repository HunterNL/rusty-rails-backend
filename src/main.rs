mod api;
mod cache;
mod iff;
mod ns_api;

use std::path::PathBuf;

use clap::{Parser, Subcommand};

static SECRET_ENV_PATH: &str = "./config/secrets.env";

#[derive(Parser)]
struct CliOptions {
    #[arg(short,long,default_value_t=String::from("./cache"))]
    cache_dir: String,

    #[command(subcommand)]
    command: SubCommand,

    #[arg(short, long)]
    allow_cache_overwrite: bool,
    // #[arg(long = "ssl")]
    // use_ssl: bool,
}

#[derive(Subcommand)]
enum SubCommand {
    Fetch,
    Serve,
}

// enum SSLConfig {
//     None,
//     Native(Vec<u8>, String),
// }

// impl SSLConfig {
//     fn from_option(enable: bool) -> SSLConfig {
//         if !enable {
//             return SSLConfig::None;
//         }
//         let id = fs::read("./key.p12").expect("key file present");
//         SSLConfig::Native(id, "".to_owned())
//     }
// }

struct AppConfig {
    ns_api_key: Option<String>,
    cache_dir: PathBuf,
    allow_cache_overwrite: bool,
}
fn main() {
    match dotenvy::from_path(SECRET_ENV_PATH) {
        Ok(()) => println!("Loaded env from {}", SECRET_ENV_PATH),
        Err(_) => println!("Skipped loading env from  {}", SECRET_ENV_PATH),
    }

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
        SubCommand::Serve => api::serve(config),
    };

    match res {
        Ok(()) => std::process::exit(0),
        Err(msg) => {
            eprintln!("{msg}");
            std::process::exit(1)
        }
    }
}
