use std::{
    fs::{create_dir_all, File},
    io::Write,
    path::Path,
    time::Duration,
};

use reqwest::blocking::Client;

use crate::AppConfig;

static TIMETABLE_CACHE: CacheItem = CacheItem {
    url: "http://data.ndovloket.nl/ns/ns-latest.zip",
    file_path: "ns-latest.zip",
};

struct CacheItem {
    url: &'static str,
    file_path: &'static str,
}

impl CacheItem {
    fn ensure_present(
        &self,
        client: Client,
        allow_overwrite: bool,
        base_dir: &Path,
    ) -> Result<(), String> {
        let file_path = base_dir.join(self.file_path);

        if file_path.exists() && !allow_overwrite {
            return Err("File already exist, pass --allow-cache-overwrite to force update".into());
        };

        let request = client
            .get(self.url)
            .build()
            .map_err(|e| format!("Error constructing HTTP request: {}", e))?;

        let response = client
            .execute(request)
            .map_err(|e| format!("Error making HTTP request: {}", e))?
            .bytes()
            .expect("Valid bytes");

        let mut file =
            File::create(file_path).map_err(|e| format!("Error making file handle: {}", e))?;

        file.write_all(&response)
            .map_err(|e| format!("Error writing response file: {}", e))
    }
}

pub(crate) fn update(config: AppConfig) -> Result<(), String> {
    if config.cache_dir.extension().is_some() {
        return Err("Expected cache_dir to be a directory".to_owned());
    }

    let client = reqwest::blocking::Client::builder()
        .connect_timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| format!("Error building client: {}", e))?;

    create_dir_all(&config.cache_dir).map_err(|e| format!("Error creating directory: {}", e))?;

    TIMETABLE_CACHE
        .ensure_present(client, config.allow_cache_overwrite, &config.cache_dir)
        .map_err(|e| format!("Error getting timetable: {}", e))?;

    Ok(())
}
