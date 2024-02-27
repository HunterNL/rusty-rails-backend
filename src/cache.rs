use std::{
    error::Error,
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
    time::Duration,
};

use crate::{
    ndovloket_api,
    ns_api::{self},
    AppConfig,
};

const TIMETABLE_PATH: &str = "ns-latest.zip";
const STATION_FILEPATH: &str = "stations.json";
const ROUTE_FILEPATH: &str = "route.json";

pub struct Cache {
    client: reqwest::blocking::Client,
    allow_overwrite: bool,
    base_dir: PathBuf,
}

impl Cache {
    pub fn new(
        allow_overwrite: bool,
        base_dir: PathBuf,
        client: Option<reqwest::blocking::Client>,
    ) -> Result<Self, String> {
        let client = client.unwrap_or_else(|| {
            reqwest::blocking::Client::builder()
                .connect_timeout(Duration::from_secs(30))
                .build()
                .expect("client") // TODO Bubble up this error
        });

        fs::create_dir_all(&base_dir).map_err(|e| e.to_string())?;

        Ok(Self { client, allow_overwrite, base_dir })
    }

    pub fn ensure_present<S>(&self, source: S, output_path: &Path) -> Result<(), String>
    where
        S: FnOnce() -> Result<Vec<u8>, Box<dyn Error>>,
    {
        let file_path = self.base_dir.join(output_path);

        if file_path.exists() && !self.allow_overwrite {
            return Err("File already exist, pass --allow-cache-overwrite to force update".into());
        };

        let mut file =
            File::create(&file_path).map_err(|e| format!("Error making file handle: {e}"))?;

        let content = source().map_err(|e| e.to_string())?;

        file.write_all(&content)
            .map_err(|e| format!("Error writing response file: {e}"))
    }
}

pub fn update(config: AppConfig) -> Result<(), String> {
    let storage_dir = config.cache_dir.join("remote");

    if config.cache_dir.extension().is_some() {
        return Err("Expected cache_dir to be a directory".to_owned());
    }

    let cache = Cache::new(config.allow_cache_overwrite, storage_dir, None)?;

    cache
        .ensure_present(
            ndovloket_api::NDovLoket::fetch_timetable,
            Path::new(TIMETABLE_PATH),
        )
        .unwrap_or_else(|e| eprintln!("{e}"));

    if config.ns_api_key.is_some() {
        let ns = ns_api::NsApi::new(config.ns_api_key.unwrap());

        cache
            .ensure_present(
                || ns.fetch_routes().map_err(|e| Box::new(e) as Box<dyn Error>),
                Path::new(ROUTE_FILEPATH),
            )
            .unwrap_or_else(|e| eprintln!("{e}"));
        cache
            .ensure_present(
                || {
                    ns.fetch_stations()
                        .map_err(|e| Box::new(e) as Box<dyn Error>)
                },
                Path::new(STATION_FILEPATH),
            )
            .unwrap_or_else(|e| eprintln!("{e}"));
    } else {
        println!("Skipping updating NS data, no key");
    }

    Ok(())
}
