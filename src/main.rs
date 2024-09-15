mod access;
mod api;
mod cache;
mod cli;
mod contextual_serializer;
mod dayoffset;
// mod experiment;
mod fetch;
mod iff;
mod ndovloket_api;
mod print;
mod time;

use std::{path::PathBuf, time::Instant};

use anyhow::{Context, Ok};

use api::datarepo::{self, DataRepo};
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

fn wait_user_input() {
    println!("Waiting for user input");
    let mut dummy = String::new();
    std::io::stdin().read_line(&mut dummy).unwrap();
}

fn main() -> Result<(), anyhow::Error> {
    println!("Main");
    let config: Figment = Figment::new()
        .merge(Toml::file("./config/project.toml"))
        .merge(Toml::file("./config/project.secret.toml"))
        .merge(Toml::file("./config/local.toml"))
        .merge(Env::prefixed("APP_")); // For deployment?

    let config: AppConfig = config.extract().context("Parsing config files")?;
    let cli_options = cli::get_cli_args();

    println!("Config was read");

    // wait_user_input();

    // config.allow_cache_overwrite = cli_options.allow_cache_overwrite;
    // config.cache_dir = cli_options.cache_dir.into();

    match cli_options.command {
        cli::SubCommand::Fetch => fetch::fetch(&config.cache_dir, config.ns_api_key.as_deref()),
        cli::SubCommand::Serve { autofetch } => api::serve(&config, autofetch),
        cli::SubCommand::Verify => verify(&config),
        cli::SubCommand::Print(args) => print::print(&config, args),
        cli::SubCommand::Bench => benchparser(&config),
    }
}

fn benchparser(config: &AppConfig) -> Result<(), anyhow::Error> {
    let start = Instant::now();
    let _ = datarepo::DataRepo::new(&config.cache_dir);
    let end = Instant::now();

    println!(
        "Building datarepo took {}ms",
        end.duration_since(start).as_millis()
    );

    Ok(())
}

fn verify(config: &AppConfig) -> Result<(), anyhow::Error> {
    DataRepo::new(&config.cache_dir).report_unkown_legs();

    Ok(())
}
