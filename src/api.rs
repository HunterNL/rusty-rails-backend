use poem::{
    endpoint::StaticFileEndpoint, get, handler, listener::TcpListener, web::Path, Route, Server,
};

mod datarepo;

use crate::AppConfig;

#[handler]
fn hello(Path(name): Path<String>) -> String {
    format!("hello: {}", name)
}

pub(crate) fn serve(config: AppConfig) -> Result<(), String> {
    let data = datarepo::DataRepo::new(&config.cache_dir);
    Ok(())
    // start_server(config)
}

#[tokio::main]
async fn start_server(config: AppConfig) -> Result<(), Box<dyn std::error::Error>> {
    let stations_endpoint = StaticFileEndpoint::new("data/stations.json");
    let links_endpoint = StaticFileEndpoint::new("data/links.json");
    let app = Route::new()
        .at("/hello/:name", get(hello))
        .at("/data/stations.json", stations_endpoint)
        .at("/data/links.json", links_endpoint);

    // let server = Server::new(TcpListener::bind("localhost:3000"));
    // server.run(app).await?;

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
