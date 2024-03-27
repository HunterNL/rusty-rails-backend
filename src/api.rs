mod datarepo;

use std::{fs, path::Path, sync::Arc};

use anyhow::Ok;
use poem::{
    endpoint::StaticFileEndpoint,
    get, handler,
    http::header,
    listener::TcpListener,
    middleware::{AddData, Cors},
    web::Data,
    EndpointExt, Response, Route, Server,
};
use serde::{ser::SerializeStruct, Serialize};
use tokio::sync::mpsc;

use crate::{
    iff::{Leg, LegKind, Record, StopKind},
    AppConfig,
};

use self::datarepo::DataRepo;

pub struct ApiObject<'a, T: ?Sized>(&'a T);

pub trait IntoAPIObject {
    fn as_api_object(&self) -> ApiObject<'_, Self> {
        ApiObject(self)
    }
}

impl IntoAPIObject for Record {}
impl IntoAPIObject for Leg {}

fn stopkind_to_num(stop_kind: &StopKind) -> u8 {
    match stop_kind {
        StopKind::Waypoint => 1,
        StopKind::StopShort(_, _) => 2,
        StopKind::StopLong(_, _, _) => 3,
        StopKind::Departure(_, _) => 4,
        StopKind::Arrival(_, _) => 5,
    }
}

impl<'a> Serialize for ApiObject<'a, Leg> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut leg = serializer.serialize_struct("leg", 9)?;
        leg.serialize_field("timeStart", &self.0.start)?;
        leg.serialize_field("timeEnd", &self.0.end)?;
        leg.serialize_field("moving", &self.0.kind.is_moving())?;
        leg.serialize_field("waypoints", &self.0.kind.waypoints())?;
        leg.serialize_field("from", &self.0.kind.from())?;
        leg.serialize_field("to", &self.0.kind.to())?;
        leg.serialize_field("stationCode", &self.0.kind.station_code())?;
        leg.serialize_field("platform", &self.0.kind.platform_info())?;

        let stoptype = match &self.0.kind {
            LegKind::Stationary(_, stop_kind) => Some(stopkind_to_num(stop_kind)),
            LegKind::Moving(_, _, _) => None,
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
        let mut ride = serializer.serialize_struct("ride", 7)?;
        ride.serialize_field("id", &self.0.id)?;
        ride.serialize_field("startTime", &self.0.start_time())?;
        ride.serialize_field("endTime", &self.0.end_time())?;
        ride.serialize_field("distance", &0)?;
        ride.serialize_field("dayValidity", &0)?;
        ride.serialize_field("rideIds", &self.0.ride_id)?;
        ride.serialize_field(
            "legs",
            &self
                .0
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

#[handler]
fn hello(poem::web::Path(name): poem::web::Path<String>) -> String {
    format!("hello: {name}")
}

pub fn serve(config: AppConfig) -> Result<(), anyhow::Error> {
    let http_dir = config.cache_dir.join(HTTP_CACHE_SUBDIR);
    let data = datarepo::DataRepo::new(&config.cache_dir);
    prepare_files(&data, &http_dir)?;

    start_server(config, data)
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

#[handler]
fn active_rides_endpoint(data: Data<&Arc<DataRepo>>, _req: String) -> Response {
    let timetable_tz = chrono_tz::Europe::Amsterdam;

    let now = chrono::Utc::now().with_timezone(&timetable_tz);

    let data = data.as_ref();
    let rides = data.rides_active_at_time(&now.naive_local().time(), &now.date_naive());

    let v: Vec<_> = rides.iter().map(|r| r.as_api_object()).collect();

    let data = serde_json::to_vec(&v).unwrap();

    Response::builder()
        .header(header::CONTENT_TYPE, "application/json; charset=utf-8")
        .body(data)
}

#[tokio::main]
async fn start_server(config: AppConfig, data: DataRepo) -> Result<(), anyhow::Error> {
    let (shutdown_sender, mut shutdown_receiver) = mpsc::channel(1);

    ctrlc::set_handler(move || {
        println!("Shutdown signal received");
        shutdown_sender
            .try_send(())
            .expect("Error sending shutdown signal");
    })
    .expect("Error setting Ctrl+C handler");

    let api_data = Arc::new(data);
    let https_serve_dir = config.cache_dir.join(HTTP_CACHE_SUBDIR);
    let stations_endpoint = StaticFileEndpoint::new(https_serve_dir.join(HTTP_CACHE_STATION_PATH));
    let links_endpoint = StaticFileEndpoint::new(https_serve_dir.join(HTTP_CACHE_LINK_PATH));

    let cors = Cors::new().allow_origins(["https://localhost:3000", "https://127.0.0.1:3000"]);

    let app = Route::new()
        .at("/hello/:name", get(hello))
        .at("/data/stations.json", stations_endpoint)
        .at("/data/links.json", links_endpoint)
        .at("/api/activerides", active_rides_endpoint)
        .with(AddData::new(api_data))
        .with(cors);

    let server = Server::new(TcpListener::bind("localhost:9001"));

    println!("Server starting");

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
