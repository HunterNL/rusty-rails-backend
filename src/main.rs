mod access;
mod api;
mod cache;
mod cli;
mod dayoffset;
mod fetch;
mod iff;
mod ndovloket_api;
mod print;
mod time;

use std::{env, path::PathBuf};

use anyhow::{Context, Ok};

use api::datarepo::DataRepo;
use figment::{
    providers::{Env, Format, Toml},
    Figment,
};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct AppConfig {
    pub ns_api_key: Option<String>,
    pub cache_dir: PathBuf,
    pub allow_cache_overwrite: bool,
    pub cors_domain: String,
    pub bind_addr: String,
}

fn main() -> Result<(), anyhow::Error> {
    let config = Figment::new()
        .merge(Toml::file("./config/project.toml"))
        .merge(Toml::file("./config/project.secret.toml"))
        .merge(Toml::file("./config/local.toml"))
        .merge(Env::prefixed("APP_")); // For deployment?

    let cur = env::current_dir().unwrap();
    println!("pwd: {}", cur.to_str().unwrap());
    let config: AppConfig = config.extract().context("Parsing config files")?;
    let cli_options = cli::get_cli_args();

    // config.allow_cache_overwrite = cli_options.allow_cache_overwrite;
    // config.cache_dir = cli_options.cache_dir.into();

    match cli_options.command {
        cli::SubCommand::Fetch => fetch::fetch(&config.cache_dir, config.ns_api_key.as_deref()),
        cli::SubCommand::Serve { autofetch } => api::serve(&config, autofetch),
        cli::SubCommand::Verify => verify(&config),
        cli::SubCommand::Print(args) => Ok(print::print(&config, args)),
    }
}

fn verify(config: &AppConfig) -> Result<(), anyhow::Error> {
    DataRepo::new(&config.cache_dir).report_unkown_legs();

    Ok(())
}
