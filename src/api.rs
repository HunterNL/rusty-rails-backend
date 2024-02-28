mod datarepo;

use std::{fs, path::Path, sync::Arc};

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

use crate::{iff::Record, AppConfig};

use self::datarepo::DataRepo;

pub struct ApiObject<'a, T: ?Sized>(&'a T);

pub trait IntoAPIObject {
    fn as_api_object(&self) -> ApiObject<'_, Self> {
        ApiObject(self)
    }
}

impl IntoAPIObject for Record {}

impl<'a> Serialize for ApiObject<'a, Record> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut ride = serializer.serialize_struct("ride", 10)?;
        ride.serialize_field("id", &self.0.id)?;
        ride.serialize_field("startTime", &self.0.start_time())?;
        ride.serialize_field("endTime", &self.0.end_time())?;
        ride.serialize_field("distance", &0)?;
        ride.serialize_field("dayValidity", &0)?;
        ride.serialize_field("legs", &self.0.generate_legs())?;
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

pub fn serve(config: AppConfig) -> Result<(), String> {
    let http_dir = config.cache_dir.join(HTTP_CACHE_SUBDIR);
    let data = datarepo::DataRepo::new(&config.cache_dir);
    prepare_files(&data, &http_dir).map_err(|()| "Error preparing files".to_owned())?;

    start_server(config, data).map_err(|e| e.to_string())
}

fn prepare_files(data: &DataRepo, http_cache_dir: &Path) -> Result<(), ()> {
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
async fn start_server(config: AppConfig, data: DataRepo) -> Result<(), Box<dyn std::error::Error>> {
    let d = Arc::new(data);
    let https_serve_dir = config.cache_dir.join(HTTP_CACHE_SUBDIR);
    let stations_endpoint = StaticFileEndpoint::new(https_serve_dir.join(HTTP_CACHE_STATION_PATH));
    let links_endpoint = StaticFileEndpoint::new(https_serve_dir.join(HTTP_CACHE_LINK_PATH));

    let cors = Cors::new().allow_origins(["https://localhost:3000", "https://127.0.0.1:3000"]);

    let app = Route::new()
        .at("/hello/:name", get(hello))
        .at("/data/stations.json", stations_endpoint)
        .at("/data/links.json", links_endpoint)
        .at("/api/activerides", active_rides_endpoint)
        .with(AddData::new(d))
        .with(cors);

    let server = Server::new(TcpListener::bind("localhost:9001"));

    server.run(app).await?;

    Ok(())
}
