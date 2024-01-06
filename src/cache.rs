mod ns_api;

use std::{
    fs::{self, File},
    io::Write,
    path::Path,
    time::Duration,
};

use reqwest::blocking::Client;

use crate::AppConfig;

use self::ns_api::NsApi;

static TIMETABLE_CACHE: CacheItem = CacheItem {
    url: "http://data.ndovloket.nl/ns/ns-latest.zip",
    file_path: "ns-latest.zip",
};

static STATION_FILEPATH: &str = "stations.json";
static ROUTE_FILEPATH: &str = "route.json";

struct CacheItem {
    url: &'static str,
    file_path: &'static str,
}

// enum UpdateResult {
//     Ok,
//     Skippped,
//     Error(String),
// }

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

    fs::create_dir_all(&config.cache_dir)
        .map_err(|e| format!("Error creating cache directory: {}", e))?;

    match update_timetable(&config) {
        Ok(_) => println!("Timetable updated"),
        Err(e) => println!("Error updating timetable: {}", e),
    }

    if config.ns_api_key.is_some() {
        update_ns_data(&config);
    } else {
        println!("Skipping NS data, no API key provided");
    }

    Ok(())
}

fn update_ns_data(config: &AppConfig) {
    let ns_api = NsApi::new(config.ns_api_key.as_ref().unwrap().clone());

    if let Some(e) = update_station_data(config, &ns_api).err() {
        println!("{}", e)
    }
    if let Some(e) = update_route_data(config, &ns_api).err() {
        println!("{}", e)
    }
}

fn update_route_data(config: &AppConfig, ns_api: &NsApi) -> Result<(), String> {
    let filepath = config.cache_dir.join(ROUTE_FILEPATH);
    if !filepath.exists() || config.allow_cache_overwrite {
        let mut file = File::create(filepath).map_err(|e| format!("Error creating file: {}", e))?;

        let data = ns_api
            .routes()
            .map_err(|e| format!("Error getting station data: {}", e))?;

        file.write_all(&data)
            .map_err(|e| format!("Error writing data: {}", e))?;
    };

    Ok(())
}

fn update_station_data(config: &AppConfig, ns_api: &NsApi) -> Result<(), String> {
    let filepath = config.cache_dir.join(STATION_FILEPATH);
    if !filepath.exists() || config.allow_cache_overwrite {
        let mut file = File::create(filepath).map_err(|e| format!("Error creating file: {}", e))?;

        let data = ns_api
            .stations()
            .map_err(|e| format!("Error getting station data file: {}", e))?;

        file.write_all(&data)
            .map_err(|e| format!("Error writing file: {}", e))?;
    }

    Ok(())
}

fn update_timetable(config: &AppConfig) -> Result<(), String> {
    let client = reqwest::blocking::Client::builder()
        .connect_timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| format!("Error building client: {}", e))?;

    match TIMETABLE_CACHE.ensure_present(client, config.allow_cache_overwrite, &config.cache_dir) {
        Ok(_) => println!("Updated timetable"),
        Err(msg) => println!("Error updating timetable: {}", msg),
    };
    Ok(())
}
