pub mod datarepo;

use std::{collections::HashSet, fs, path::Path, sync::Arc};

use active_rides_timespan::active_rides_in_timespan_endpoint;
use anyhow::{anyhow, Ok};

use company_map::company_endpoint;
use find_path_endpoint::route_finding_endpoint;
use location_map::location_map_endpoint;
use ns_api::NsApi;
use poem::{
    endpoint::StaticFileEndpoint,
    get,
    listener::TcpListener,
    middleware::{AddData, CatchPanic, Cors},
    EndpointExt, Route, Server,
};
use serde::{ser::SerializeStruct, Deserialize, Serialize};
use tokio::sync::mpsc;

mod active_rides;
mod active_rides_timespan;
mod all_rides;
mod company_map;
mod errorresponse;
mod find_path_endpoint;
mod location_map;

use crate::{
    api::{active_rides::active_rides_endpoint, all_rides::all_rides_endpoint},
    fetch,
    iff::{Leg, LegKind, Record, Ride, StopKind},
    AppConfig,
};

use self::datarepo::DataRepo;

pub struct ApiObject<'a, T: ?Sized> {
    inner: &'a T,
}

pub trait IntoAPIObject {
    fn as_api_object(&self) -> ApiObject<'_, Self> {
        ApiObject { inner: self }
    }
}

impl IntoAPIObject for Record {}
impl IntoAPIObject for Leg {}
impl IntoAPIObject for Ride {}

fn stopkind_to_num(stop_kind: &StopKind) -> u8 {
    match stop_kind {
        StopKind::Waypoint => 1,
        StopKind::StopShort(_, _) => 2,
        StopKind::StopLong(_, _, _) => 3,
        StopKind::Departure(_, _) => 4,
        StopKind::Arrival(_, _) => 5,
    }
}

impl<'a, 'b> Serialize for ApiObject<'a, Leg> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut leg = serializer.serialize_struct("leg", 9)?;
        leg.serialize_field("timeStart", &self.inner.start)?;
        leg.serialize_field("timeEnd", &self.inner.end)?;
        leg.serialize_field("moving", &self.inner.kind.is_moving())?;
        leg.serialize_field("waypoints", self.inner.kind.waypoints().unwrap_or(&vec![]))?;
        leg.serialize_field("from", &self.inner.kind.from())?;
        leg.serialize_field("to", &self.inner.kind.to())?;
        leg.serialize_field("stationCode", &self.inner.kind.station_code())?;
        leg.serialize_field("platform", &self.inner.kind.platform_info())?;

        let stoptype = match &self.inner.kind {
            LegKind::Stationary(_, stop_kind) => Some(stopkind_to_num(stop_kind)),
            LegKind::Moving {
                from: _,
                to: _,
                waypoints: _,
            } => None,
        };
        leg.serialize_field("stopType", &stoptype)?;

        leg.end()
    }
}

impl<'a> Serialize for ApiObject<'a, Record> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut record = serializer.serialize_struct("ride", 7)?;
        record.serialize_field("id", &self.inner.id)?;
        record.serialize_field("startTime", &self.inner.start_time())?;
        record.serialize_field("endTime", &self.inner.end_time())?;
        record.serialize_field("distance", &0)?;
        record.serialize_field("dayValidity", &0)?;
        record.serialize_field("rideIds", &self.inner.ride_id)?;
        record.serialize_field(
            "legs",
            &self
                .inner
                .generate_legs()
                .iter()
                .map(|l| l.as_api_object())
                .collect::<Vec<_>>(),
        )?;
        record.end()
    }
}

impl<'a> Serialize for ApiObject<'a, Ride> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut ride = serializer.serialize_struct("ride", 7)?;
        ride.serialize_field("id", &self.inner.id)?;
        ride.serialize_field("operator", &self.inner.operator)?;
        ride.serialize_field("startTime", &self.inner.start_time())?;
        ride.serialize_field("endTime", &self.inner.end_time())?;
        ride.serialize_field("distance", &0)?;
        ride.serialize_field("dayValidity", &0)?;
        ride.serialize_field("id", &self.inner.id)?;
        ride.serialize_field(
            "legs",
            &self
                .inner
                .generate_legs()
                .iter()
                .map(|l| l.as_api_object())
                .collect::<Vec<_>>(),
        )?;
        ride.end()
    }
}

const HTTP_CACHE_SUBDIR: &str = "http";
const HTTP_CACHE_STATION_PATH: &str = "stations.json";
const HTTP_CACHE_LINK_PATH: &str = "links.json";

pub fn serve(config: &AppConfig, autofetch: bool) -> Result<(), anyhow::Error> {
    if autofetch {
        println!("Autofetching...");
        fetch::fetch(&config.cache_dir, config.ns_api_key.as_deref())?;
        println!("Done autofetching")
    }
    println!("Starting serve...");

    let http_dir = config.cache_dir.join(HTTP_CACHE_SUBDIR);
    let mut data = datarepo::DataRepo::new(&config.cache_dir);
    data.filter_unknown_legs();

    prepare_files(&data, &http_dir)?;

    let ns_key = config
        .ns_api_key
        .as_ref()
        .ok_or(anyhow!("NS API key missing!"))?;

    let ns_api = ns_api::NsApi::new(ns_key.to_owned());

    //Health check
    let timetable_tz = chrono_tz::Europe::Amsterdam;
    let now = chrono::Utc::now().with_timezone(&timetable_tz);
    let _ = data.rides_active_at_time(&now.naive_local().time(), &now.date_naive());

    start_server(config, data, ns_api)
}

fn prepare_files(data: &DataRepo, http_cache_dir: &Path) -> Result<(), anyhow::Error> {
    let link_file_content = serde_json::to_vec(data.links()).expect("should serialize links");
    let station_file_content =
        serde_json::to_vec(data.stations()).expect("should serialize stations");

    fs::create_dir_all(http_cache_dir).expect("Http cache dir to exist or be created");
    fs::write(
        http_cache_dir.join(HTTP_CACHE_STATION_PATH),
        station_file_content,
    )
    .expect("write stations file");
    fs::write(http_cache_dir.join(HTTP_CACHE_LINK_PATH), link_file_content)
        .expect("write links file");

    Ok(())
}

#[derive(Serialize)]
struct RoutePlannerResponse<'a> {
    /// Possible routes
    trips: Vec<RoutePlannerTrip>,
    /// All rides used in the above routes
    rides: Vec<ApiObject<'a, Ride>>,
}

#[derive(Serialize)]
struct RoutePlannerTrip {
    legs: Vec<RoutePlannerLeg>,
}

#[derive(Serialize)]
struct RoutePlannerLeg {
    from: String,
    to: String,
    id: String,
}

impl<'a> RoutePlannerResponse<'a> {
    pub fn new(res: &ns_api::Response, repo: &'a datarepo::DataRepo) -> Self {
        let now = chrono::Utc::now().with_timezone(&chrono_tz::Europe::Amsterdam);
        let trips: Vec<_> = res
            .trips
            .iter()
            .filter(|trip| {
                trip.legs.iter().all(|leg| {
                    matches!(
                        leg.travel_type,
                        ns_api::response_data::LegKind::PublicTransit
                    )
                })
            })
            .map(|trip| RoutePlannerTrip {
                legs: trip
                    .legs
                    .iter()
                    .map(|leg| RoutePlannerLeg {
                        from: leg.origin.get_code().unwrap().to_owned(),
                        to: leg.destination.get_code().unwrap().to_owned(),
                        id: leg.product.get_number().unwrap().to_owned(),
                    })
                    .collect(),
            })
            .collect();

        let trip_ids: HashSet<_> = trips
            .iter()
            .flat_map(|trip| &trip.legs)
            .map(|leg| &leg.id)
            .collect();

        Self {
            rides: repo
                .rides()
                .iter()
                .filter(|r| repo.is_ride_valid(r.day_validity, now.date_naive()))
                .filter(|ride| trip_ids.contains(&ride.id))
                .map(|r| r.as_api_object())
                .collect(),
            // rides: vec![],
            trips,
        }
    }
}

#[derive(Deserialize)]
struct PathfindingArguments {
    from: String,
    to: String,
}

impl PathfindingArguments {
    fn validate_string(s: &str, stations: &Arc<HashSet<Box<str>>>) -> bool {
        s.len() < 50 && stations.contains(s)
    }

    pub fn validate(&self, stations: &Arc<HashSet<Box<str>>>) {
        let val_a = Self::validate_string(&self.from, stations);
        let val_b = Self::validate_string(&self.to, stations);

        if !val_a || !val_b {
            panic!("Queries don't pass")
        }
    }
}

#[tokio::main]
async fn start_server(
    config: &AppConfig,
    data: DataRepo,
    ns_api: NsApi,
) -> Result<(), anyhow::Error> {
    let (shutdown_sender, mut shutdown_receiver) = mpsc::channel(1);

    ctrlc::set_handler(move || {
        println!("Shutdown signal received");
        shutdown_sender
            .try_send(())
            .expect("Error sending shutdown signal");
    })
    .expect("Error setting Ctrl+C handler");

    let https_serve_dir = config.cache_dir.join(HTTP_CACHE_SUBDIR);
    let stations_endpoint = StaticFileEndpoint::new(https_serve_dir.join(HTTP_CACHE_STATION_PATH));
    let links_endpoint = StaticFileEndpoint::new(https_serve_dir.join(HTTP_CACHE_LINK_PATH));
    let station_allowlist: HashSet<Box<str>> = data
        .stations()
        .iter()
        .map(|station| station.code.to_lowercase().into_boxed_str())
        .collect();

    let cors = Cors::new().allow_origin(&config.cors_domain);
    let catch_panic = CatchPanic::new();

    let app = Route::new()
        .at("/data/stations.json", get(stations_endpoint))
        .at("/data/links.json", get(links_endpoint))
        .at("/data/location_map.json", get(location_map_endpoint))
        .at("/data/company_map.json", get(company_endpoint))
        .at("/api/activerides", get(active_rides_endpoint))
        .at(
            "/api/activerides_timespan",
            get(active_rides_in_timespan_endpoint),
        )
        .at("/api/find_route", get(route_finding_endpoint))
        .at("/api/rides_all", get(all_rides_endpoint))
        .with(catch_panic)
        .with(cors)
        .with(AddData::new(Arc::new(data)))
        .with(AddData::new(Arc::new(ns_api)))
        .with(AddData::new(Arc::new(station_allowlist)));

    let server = Server::new(TcpListener::bind(&config.bind_addr));

    println!("CORS domains: {}", config.cors_domain);
    println!("Server starting on {}", config.bind_addr);

    server
        .run_with_graceful_shutdown(
            app,
            async {
                shutdown_receiver.recv().await;
                println!("Shutting down server")
            },
            Some(std::time::Duration::from_secs(5)),
        )
        .await?;

    println!("Server shutdown");

    Ok(())
}
