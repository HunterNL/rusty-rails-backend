mod api;
mod cache;
mod cli;
mod dayoffset;
mod iff;
mod ndovloket_api;
mod ns_api;

use std::path::PathBuf;

static SECRET_ENV_PATH: &str = "./config/secrets.env";

pub struct AppConfig {
    pub ns_api_key: Option<String>,
    pub cache_dir: PathBuf,
    pub allow_cache_overwrite: bool,
}

fn main() {
    match dotenvy::from_path(SECRET_ENV_PATH) {
        Ok(()) => println!("Loaded env from {SECRET_ENV_PATH}"),
        Err(_) => println!("Skipped loading env from  {SECRET_ENV_PATH}"),
    }

    let cli_options = cli::get_cli_args();
    let ns_key = std::env::var("NS_API_KEY");
    let cache_dir: PathBuf = cli_options.cache_dir.into();

    let config: AppConfig = AppConfig {
        cache_dir,
        ns_api_key: ns_key.ok(),
        allow_cache_overwrite: cli_options.allow_cache_overwrite,
    };

    let res: Result<(), String> = match cli_options.command {
        cli::SubCommand::Fetch => cache::update(config),
        cli::SubCommand::Serve => api::serve(config),
    };

    match res {
        Ok(()) => std::process::exit(0),
        Err(msg) => {
            eprintln!("{msg}");
            std::process::exit(1)
        }
    }
}
