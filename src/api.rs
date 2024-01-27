use std::{fs, sync::Arc};

use poem::{
    endpoint::StaticFileEndpoint,
    get, handler,
    http::header,
    listener::TcpListener,
    middleware::{AddData, Cors},
    web::{Data, Path},
    EndpointExt, Response, Route, Server,
};
use serde::{ser::SerializeStruct, Serialize};

mod datarepo;

use crate::{iff::parsing::Record, AppConfig};

use self::datarepo::DataRepo;

struct RideApi<'a> {
    record: &'a Record,
}

impl<'a> Serialize for RideApi<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut ride = serializer.serialize_struct("ride", 10)?;
        ride.serialize_field("id", &self.record.id)?;
        ride.serialize_field("stops", &self.record.timetable)?;
        ride.serialize_field("startTime", &self.record.start_time())?;
        ride.serialize_field("endTime", &self.record.end_time())?;
        ride.serialize_field("distance", &0)?;
        ride.serialize_field("dayValidity", &0)?;
        ride.serialize_field("legs", &self.record.generate_legs())?;
        ride.end()
    }
}

#[handler]
fn hello(Path(name): Path<String>) -> String {
    format!("hello: {name}")
}

pub fn serve(config: AppConfig) -> Result<(), String> {
    let data = datarepo::DataRepo::new(&config.cache_dir);
    prepare_files(&data).map_err(|()| "Error preparing files".to_owned())?;

    // println!("{:?}", data.links());

    start_server(config, data).map_err(|e| e.to_string())
}

fn prepare_files(data: &DataRepo) -> Result<(), ()> {
    let link_file_content = serde_json::to_vec(data.links()).unwrap();
    let station_file_content = serde_json::to_vec(data.stations()).unwrap();

    fs::create_dir_all("cache/http").unwrap();
    fs::write("cache/http/stations.json", station_file_content).unwrap();
    fs::write("cache/http/links.json", link_file_content).unwrap();

    Ok(())
}

// struct ApiEndpoint<'a> {
//     data: &'a DataRepo,
// }

#[handler]
fn active_rides_endpoint(data: Data<&Arc<DataRepo>>, _req: String) -> Response {
    let timetable_tz = chrono_tz::Europe::Amsterdam;
    let now = chrono::Utc::now().with_timezone(&timetable_tz);

    let data = data.as_ref();
    let rides = data.rides_active_at_time(&now.naive_local().time());

    let v: Vec<RideApi> = rides.iter().map(|r| RideApi { record: r }).collect();

    let data = serde_json::to_vec(&v).unwrap();

    // println!("{:?}", data.rides_active_at_time(&now.naive_local().time()));

    Response::builder()
        .header(header::CONTENT_TYPE, "application/json; charset=utf-8")
        .body(data)
}

#[tokio::main]
async fn start_server(_config: AppConfig, data: DataRepo) -> Result<(), Box<dyn std::error::Error>> {
    let d = Arc::new(data);
    let stations_endpoint = StaticFileEndpoint::new("cache/http/stations.json");
    let links_endpoint = StaticFileEndpoint::new("cache/http/links.json");

    let cors = Cors::new().allow_origin("https://localhost:3000");

    let app = Route::new()
        .at("/hello/:name", get(hello))
        .at("/data/stations.json", stations_endpoint)
        .at("/data/links.json", links_endpoint)
        .at("/api/activerides", active_rides_endpoint)
        .with(AddData::new(d))
        .with(cors);

    // if let SSLConfig::Native(id, password) = config.ssl {
    //     let listener2 = listener_1.native_tls(NativeTlsConfig::new().password(password).pkcs12(id));
    // } else {
    //     let listener2 = listener_1;
    // }

    let server = Server::new(TcpListener::bind("localhost:9001"));

    server.run(app).await?;

    // println!("{}", timetable.header.company_id);
    // println!("{}", timetable.header.first_valid_date);
    // println!("{}", timetable.header.last_valid_date);

    // let a: Vec<String> = timetable
    //     .rides
    //     .iter()
    //     .map(|ride| ride.timetable.first().unwrap().code.clone())
    //     .collect();

    // println!("{}", a.join(","));

    Ok(())
}
