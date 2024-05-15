use poem::{handler, http::header, Response};

use std::sync::Arc;

use poem::web::Data;

use crate::api::{datarepo::DataRepo, IntoAPIObject};

#[handler]
pub fn active_rides_endpoint(data: Data<&Arc<DataRepo>>, _req: String) -> Response {
    let timetable_tz = chrono_tz::Europe::Amsterdam;

    let now = chrono::Utc::now().with_timezone(&chrono_tz::Europe::Amsterdam);

    let data = data.as_ref();
    let rides = data.rides_active_at_time(&now.naive_local().time(), &now.date_naive());

    let v: Vec<_> = rides.iter().map(|r| r.as_api_object()).collect();

    let data = serde_json::to_vec(&v).unwrap();

    Response::builder()
        .header(header::CONTENT_TYPE, "application/json; charset=utf-8")
        .body(data)
}
