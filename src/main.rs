#![feature(array_windows)]

mod access;
mod api;
mod cache;
mod cli;
mod dayoffset;
mod fetch;
mod iff;
mod ndovloket_api;

use std::path::PathBuf;

use anyhow::Context;

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
}

fn main() -> Result<(), anyhow::Error> {
    let config = Figment::new()
        .merge(Toml::file("./config/project.toml"))
        .merge(Toml::file("./config/project.secret.toml"))
        .merge(Toml::file("./config/local.toml"))
        .merge(Env::prefixed("APP_")); // For deployment?

    let config: AppConfig = config.extract().context("Parsing config files")?;

    let cli_options = cli::get_cli_args();
    let cache_dir: PathBuf = cli_options.cache_dir.into();

    let config: AppConfig = AppConfig {
        cache_dir,
        ns_api_key: config.ns_api_key,
        allow_cache_overwrite: cli_options.allow_cache_overwrite,
    };

    match cli_options.command {
        cli::SubCommand::Fetch => fetch::fetch(config),
        cli::SubCommand::Serve => api::serve(config),
    }
}
