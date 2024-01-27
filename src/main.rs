mod api;
mod cache;
mod iff;

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
    match dotenvy::from_path("./config/secrets.env") {
        Ok(()) => {}
        Err(_) => println!("Expected to find config/secrets.env"),
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
        SubCommand::Serve => api::serve(config).map_err(|e| e.to_string()),
    };

    match res {
        Ok(_) => std::process::exit(0),
        Err(msg) => {
            eprintln!("{}", msg);
            std::process::exit(1)
        }
    }
}
