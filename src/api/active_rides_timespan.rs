use chrono::Duration;
use poem::{handler, http::header, Response};

use std::sync::Arc;

use poem::web::Data;

use crate::api::{datarepo::DataRepo, IntoAPIObject};

#[handler]
pub fn active_rides_in_timespan_endpoint(data: Data<&Arc<DataRepo>>, _req: String) -> Response {
    let start = chrono::Utc::now().with_timezone(&chrono_tz::Europe::Amsterdam);
    let end = start + Duration::hours(2); // TODO use arg instead

    let rides: Vec<_> = data
        .as_ref()
        .rides_active_in_timespan(
            &start.naive_local().time(),
            &end.naive_local().time(),
            &start.date_naive(),
        )
        .iter()
        .map(|r| r.as_api_object())
        .collect();

    let data = serde_json::to_vec(&rides).unwrap();

    Response::builder()
        .header(header::CONTENT_TYPE, "application/json; charset=utf-8")
        .body(data)
}
