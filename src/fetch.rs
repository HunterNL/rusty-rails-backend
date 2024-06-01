use anyhow::anyhow;
use std::error::Error as StdError;

use crate::{
    ndovloket_api::{self},
    AppConfig,
};

use crate::cache::{Action, Cache, CacheError};

const TIMETABLE_PATH: &str = "ns-latest.zip";
const STATION_FILEPATH: &str = "stations.json";
const ROUTE_FILEPATH: &str = "route.json";

fn print_cacheresult<E: StdError>(res: Result<Action, CacheError<E>>) {
    match res {
        Ok(ok) => println!("{ok:?}"),
        Err(err) => print!("{err:?}"),
    }
}

#[tokio::main]
pub async fn fetch(config: &AppConfig) -> Result<(), anyhow::Error> {
    let storage_dir = config.cache_dir.join("remote");

    if config.cache_dir.extension().is_some() {
        return Err(anyhow!("Expected cache_dir to be a directory"));
    }

    let cache = Cache::new(storage_dir)?;

    let timetable_result = cache
        .ensure_async(ndovloket_api::NDovLoket::fetch_timetable, TIMETABLE_PATH)
        .await;

    print_cacheresult(timetable_result);

    if let Some(key) = &config.ns_api_key {
        let ns = ns_api::NsApi::new(key.clone());

        let a = cache
            .ensure_async(|| ns.fetch_routes(), ROUTE_FILEPATH)
            .await;

        print_cacheresult(a);

        let b = cache
            .ensure_async(|| ns.fetch_stations(), STATION_FILEPATH)
            .await;

        print_cacheresult(b);
    } else {
        println!("Skipping updating NS data, no key");
    }

    Ok(())
}
