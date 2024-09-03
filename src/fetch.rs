use anyhow::anyhow;
use std::path::Path;

use crate::{
    iff,
    ndovloket_api::{self},
};

use crate::cache::{Action, Cache, Error};

pub static TIMETABLE_PATH: &str = "remote/ns_iff.zip";
pub const STATION_FILEPATH: &str = "remote/stations.json";
pub const ROUTE_FILEPATH: &str = "remote/route.json";

fn print_cacheresult(res: Result<Action, Error>) {
    match res {
        Ok(ok) => println!("{ok:?}"),
        Err(err) => print!("{err:?}"),
    }
}

fn is_update_required(old: &[u8], new: &[u8]) -> Result<bool, Box<dyn std::error::Error>> {
    let old_version = iff::Iff::parse_version_only(old).map_err(|_| anyhow!(" version old"))?;
    let new_version = iff::Iff::parse_version_only(new).map_err(|_| anyhow!(" version new"))?;

    Ok(old_version < new_version)
}

#[tokio::main]
pub async fn fetch(storage_dir: &Path, ns_key: Option<&str>) -> Result<(), anyhow::Error> {
    if !storage_dir.is_dir() {
        return Err(anyhow!("Expected cache_dir to be a directory"));
    }

    let cache = Cache::new(storage_dir)?;

    // let filepath = cache.base_dir.join(TIMETABLE_PATH).canonicalize().unwrap();

    // filepath.components().for_each(|c| println!("{:?}", c));

    // return Ok(());

    println!(
        "post cache init {}",
        cache.base_dir.join(TIMETABLE_PATH).display()
    );

    // return Ok((()));

    let timetable_result = cache
        .ensure_versioned_async(
            ndovloket_api::NDovLoket::fetch_timetable,
            TIMETABLE_PATH,
            is_update_required,
        )
        .await;

    print_cacheresult(timetable_result);

    if let Some(key) = ns_key {
        let ns = ns_api::NsApi::new(key.to_owned());

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
